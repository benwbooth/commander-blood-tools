//! Cyclic vertex-wave animation and camera-relative model selection.

use std::fmt;

use commander_blood_formats::alien::{AXIS_COUNT, AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT};

use super::{AlienModelPose, AlienSpecies};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const FIRST_NODE: usize = 0;
const SELECTION_COUNTER_AXIS: usize = 1;
const INITIAL_PRIMARY_PHASE: u16 = 4;
const INITIAL_PRIMARY_STEP: i16 = 48;
const INITIAL_SECONDARY_PHASE: u16 = 4;
const INITIAL_SECONDARY_STEP: i16 = 16;
const INITIAL_RADIAL_OFFSET: i16 = 12;
const ACCELERATED_PRIMARY_STEP: i16 = 368;
const PRIMARY_STEP_DECAY: i16 = 4;
const SAMPLE_PHASE_MASK: u16 = 0x0ffc;
const SAMPLE_INDEX_SHIFT: u32 = 2;
const SAMPLE_AMPLITUDE_SHIFT: u32 = 8;
const SECONDARY_PRODUCT_SHIFT: u32 = 17;
const OBJECT_PHASE_SCALE: u16 = 2;
const DISTANCE_ORIGIN: i16 = 25;
const DISTANCE_DEAD_ZONE: i16 = 50;
const SELECTION_VERTICAL_ORIGIN: i16 = 60;
const SELECTION_VERTICAL_MINIMUM: i16 = 0;
const SELECTION_VERTICAL_MAXIMUM: i16 = 128;
const SELECTION_LATERAL_LIMIT: i16 = 256;

/// Camera-selection lifecycle shared by all wave models in one alien scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlienWaveSelection {
    /// Selection checks are inactive.
    #[default]
    Disabled,
    /// A camera-relative selection check is pending.
    Requested,
    /// One model satisfied every selection bound.
    Selected,
}

/// Per-model phase state for the recovered wave method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienWaveMethodState {
    /// Whether the model has completed its one-time initialization.
    pub initialized: bool,
    /// Primary cyclic sample phase.
    pub primary_phase: u16,
    /// Signed primary phase advance.
    pub primary_step: i16,
    /// Distance-weighted secondary sample phase.
    pub secondary_phase: u16,
    /// Signed secondary phase advance.
    pub secondary_step: i16,
}

/// Scene-wide selection and sample output produced by wave models.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienWaveSceneState {
    /// Current selection lifecycle.
    pub selection: AlienWaveSelection,
    /// Model selected by the most recent successful bounds check.
    pub selected_model: Option<usize>,
    /// Primary cosine sample published for callback behavior.
    pub current_sample: i16,
}

/// Stage completed by one invocation of the wave method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienWaveUpdate {
    /// One-time model state was initialized; no vertices advanced.
    Initialized,
    /// Primary and secondary vertex waves advanced.
    Advanced,
}

/// Invalid typed model state supplied to the wave method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienWaveError {
    /// Every wave model owns a primary node.
    MissingPrimaryNode,
    /// An active wave model requires at least one authored vertex.
    EmptyVertexList,
    /// The authored vertex boundary exceeds the mutable object-position array.
    InvalidAuthoredVertexCount {
        /// Authored vertices requested by the model.
        authored: usize,
        /// Mutable object positions available in the pose.
        available: usize,
    },
}

impl fmt::Display for AlienWaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien wave state: {self:?}")
    }
}

impl std::error::Error for AlienWaveError {}

/// Initialize or advance one model's cyclic vertex-wave behavior.
pub fn update_or_initialize_wave(
    species: AlienSpecies,
    model_index: usize,
    pose: &mut AlienModelPose,
    state: &mut AlienWaveMethodState,
    scene: &mut AlienWaveSceneState,
    view: [i16; AXIS_COUNT],
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> Result<AlienWaveUpdate, AlienWaveError> {
    let node = pose
        .nodes
        .get_mut(FIRST_NODE)
        .ok_or(AlienWaveError::MissingPrimaryNode)?;
    if !state.initialized {
        state.initialized = true;
        state.primary_phase = INITIAL_PRIMARY_PHASE;
        state.primary_step = INITIAL_PRIMARY_STEP;
        state.secondary_phase = INITIAL_SECONDARY_PHASE;
        state.secondary_step = INITIAL_SECONDARY_STEP;
        scene.selection = AlienWaveSelection::Disabled;
        node.radial_offset = INITIAL_RADIAL_OFFSET;
        node.angles.fill(u16::MIN);
        if species == AlienSpecies::Scrut {
            scene.selected_model = Some(model_index);
        }
        return Ok(AlienWaveUpdate::Initialized);
    }

    if pose.authored_vertex_count == usize::MIN {
        return Err(AlienWaveError::EmptyVertexList);
    }
    if pose.authored_vertex_count > pose.object_positions.len() {
        return Err(AlienWaveError::InvalidAuthoredVertexCount {
            authored: pose.authored_vertex_count,
            available: pose.object_positions.len(),
        });
    }

    node.angles[SELECTION_COUNTER_AXIS] = node.angles[SELECTION_COUNTER_AXIS].wrapping_add(1);
    scene.current_sample =
        sample_at_phase(trigonometry, state.primary_phase) >> SAMPLE_AMPLITUDE_SHIFT;
    if scene.selection == AlienWaveSelection::Requested
        && model_is_selected(node.local_position, view, scene.current_sample)
    {
        scene.selection = AlienWaveSelection::Selected;
        scene.selected_model = Some(model_index);
        state.primary_step = ACCELERATED_PRIMARY_STEP;
    }

    if state.primary_step > INITIAL_PRIMARY_STEP {
        state.primary_step = state.primary_step.wrapping_sub(PRIMARY_STEP_DECAY);
    }
    let primary_phase = state.primary_phase;
    state.primary_phase = state.primary_phase.wrapping_add(state.primary_step as u16);
    let positions = &mut pose.object_positions[..pose.authored_vertex_count];
    for position in positions.iter_mut() {
        let object_phase = (position[Z_AXIS] as u16).wrapping_mul(OBJECT_PHASE_SCALE);
        let first_phase = object_phase.wrapping_add(primary_phase);
        let first_sample = sample_at_phase(trigonometry, first_phase) >> SAMPLE_AMPLITUDE_SHIFT;
        position[Y_AXIS] = position[Y_AXIS].wrapping_sub(first_sample);
        let second_phase = first_phase.wrapping_add(state.primary_step as u16);
        let second_sample = sample_at_phase(trigonometry, second_phase) >> SAMPLE_AMPLITUDE_SHIFT;
        position[Y_AXIS] = position[Y_AXIS].wrapping_add(second_sample);
    }

    state.secondary_phase = state
        .secondary_phase
        .wrapping_add(state.secondary_step as u16);
    for position in positions {
        let mut distance = position[X_AXIS].wrapping_sub(DISTANCE_ORIGIN);
        if distance.is_negative() {
            distance = distance.wrapping_neg().wrapping_sub(DISTANCE_DEAD_ZONE);
        }
        if distance.is_negative() {
            continue;
        }
        let scale = (distance as u16).wrapping_mul(OBJECT_PHASE_SCALE);
        let first_phase = scale.wrapping_add(state.secondary_phase);
        let first_delta = scaled_sample(trigonometry, first_phase, scale);
        position[Y_AXIS] = position[Y_AXIS].wrapping_sub(first_delta);
        let second_phase = first_phase.wrapping_add(state.secondary_step as u16);
        let second_delta = scaled_sample(trigonometry, second_phase, scale);
        position[Y_AXIS] = position[Y_AXIS].wrapping_add(second_delta);
    }
    Ok(AlienWaveUpdate::Advanced)
}

fn model_is_selected(
    local_position: [i32; AXIS_COUNT],
    view: [i16; AXIS_COUNT],
    current_sample: i16,
) -> bool {
    let vertical = current_sample
        .wrapping_sub(SELECTION_VERTICAL_ORIGIN)
        .wrapping_add(local_position[Y_AXIS] as i16)
        .wrapping_add(view[Y_AXIS]);
    let horizontal = (local_position[X_AXIS] as i16).wrapping_add(view[X_AXIS]);
    let depth = (local_position[Z_AXIS] as i16).wrapping_add(view[Z_AXIS]);
    (SELECTION_VERTICAL_MINIMUM..=SELECTION_VERTICAL_MAXIMUM).contains(&vertical)
        && (-SELECTION_LATERAL_LIMIT..=SELECTION_LATERAL_LIMIT).contains(&horizontal)
        && (-SELECTION_LATERAL_LIMIT..=SELECTION_LATERAL_LIMIT).contains(&depth)
}

fn sample_at_phase(
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    phase: u16,
) -> i16 {
    let index = usize::from((phase & SAMPLE_PHASE_MASK) >> SAMPLE_INDEX_SHIFT);
    trigonometry[index].cosine
}

fn scaled_sample(
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    phase: u16,
    scale: u16,
) -> i16 {
    i32::from(sample_at_phase(trigonometry, phase))
        .wrapping_mul(i32::from(scale))
        .wrapping_shr(SECONDARY_PRODUCT_SHIFT) as i16
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienFaceData, AlienNodeParent, AlienTransformData};
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::native::alien::{AlienNodePose, AlienProjectedVertex};

    const ORACLE_IMAGE_BYTE_COUNT: usize = 65_536;
    const ORACLE_OBJECT_BASE: usize = 0x4000;
    const ORACLE_OBJECT_BYTE_COUNT: usize = 20;
    const ORACLE_DISTANCE_FIELD: usize = 4;
    const ORACLE_MOTION_FIELD: usize = 6;
    const ORACLE_PHASE_FIELD: usize = 8;
    const ORACLE_TRIGONOMETRY_BASE: usize = 0x0036;
    const ORACLE_TRIGONOMETRY_BYTE_COUNT: usize = 4_096;
    const ORACLE_WORD_BYTE_COUNT: usize = 2;
    const ORACLE_PATTERN_MASK: usize = 0xff;
    const ORACLE_GAME_PATTERN_STRIDE: usize = 31;
    const ORACLE_OBJECT_PATTERN_STRIDE: usize = 37;
    const ORACLE_GAME_CASE_STEP: usize = 17;
    const ORACLE_OBJECT_CASE_STEP: usize = 23;
    const ORACLE_GAME_BIAS: usize = 5;
    const ORACLE_OBJECT_BIAS: usize = 7;
    const ORACLE_PRIMARY_PHASE_BASE: u16 = 0x0ff0;
    const ORACLE_PRIMARY_PHASE_STEP: u16 = 4;
    const ORACLE_SECONDARY_PHASE_BASE: u16 = 0x0ff4;
    const ORACLE_SECONDARY_PHASE_STEP: u16 = 8;
    const ORACLE_SECONDARY_STEP_BASE: i16 = 16;
    const ORACLE_SECONDARY_STEP_STEP: i16 = 4;
    const ORACLE_CURRENT_SAMPLE: i16 = 64;
    const ORACLE_PRIMARY_SAMPLE: u16 = 0x4000;
    const ZERO_VIEW_COMPONENT: i16 = 0;
    const AUTHORED_VERTEX_COUNT: usize = 3;
    const PRESERVED_SELECTED_MODEL: usize = 999;
    const INITIAL_OBJECTS: [[i16; AXIS_COUNT]; AUTHORED_VERTEX_COUNT] = [
        [40, 0x7ff8, 3],
        [-40, 0x8008_u16 as i16, 0x07ff],
        [0, 0xfff8_u16 as i16, -1],
    ];

    #[derive(Clone, Copy)]
    struct UpdateCase {
        selection: AlienWaveSelection,
        local_position: [i32; AXIS_COUNT],
        primary_step: i16,
        selected: bool,
    }

    const UPDATE_CASES: [UpdateCase; 10] = [
        UpdateCase {
            selection: AlienWaveSelection::Disabled,
            local_position: [0, 0, 0],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Disabled,
            local_position: [0, 0, 0],
            primary_step: 52,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [0, -5, 0],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [0, 125, 0],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [-257, 0, 0],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [257, 0, 0],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [0, 0, -257],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [0, 0, 257],
            primary_step: 48,
            selected: false,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [-256, -4, -256],
            primary_step: 48,
            selected: true,
        },
        UpdateCase {
            selection: AlienWaveSelection::Requested,
            local_position: [256, 124, 256],
            primary_step: 48,
            selected: true,
        },
    ];

    #[derive(Deserialize)]
    struct WaveVector {
        name: String,
        module: String,
        publishes_initial_state: Option<bool>,
        selection_after: Option<u16>,
        primary_step_after: Option<u16>,
        primary_phase_after: Option<u16>,
        secondary_phase_after: Option<u16>,
        object_data_sha256: Option<String>,
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn selection(value: u16) -> AlienWaveSelection {
        match value {
            0 => AlienWaveSelection::Disabled,
            1 => AlienWaveSelection::Requested,
            2 => AlienWaveSelection::Selected,
            _ => panic!("unknown wave selection state {value}"),
        }
    }

    fn put_word(bytes: &mut [u8], position: usize, value: u16) {
        bytes[position..position + ORACLE_WORD_BYTE_COUNT].copy_from_slice(&value.to_le_bytes());
    }

    fn get_word(bytes: &[u8], position: usize) -> u16 {
        u16::from_le_bytes(
            bytes[position..position + ORACLE_WORD_BYTE_COUNT]
                .try_into()
                .unwrap(),
        )
    }

    fn patterned_image(seed: usize, stride: usize) -> Vec<u8> {
        (usize::MIN..ORACLE_IMAGE_BYTE_COUNT)
            .map(|position| ((position * stride + seed) & ORACLE_PATTERN_MASK) as u8)
            .collect()
    }

    fn trigonometry(
        case_index: usize,
        primary_phase: u16,
    ) -> [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT] {
        let mut game = patterned_image(
            case_index * ORACLE_GAME_CASE_STEP + ORACLE_GAME_BIAS,
            ORACLE_GAME_PATTERN_STRIDE,
        );
        for position in (usize::MIN..ORACLE_TRIGONOMETRY_BYTE_COUNT).step_by(ORACLE_WORD_BYTE_COUNT)
        {
            let value = (position * 73 + case_index * 0x1111 + 0x8123) as u16;
            put_word(&mut game, ORACLE_TRIGONOMETRY_BASE + position, value);
        }
        put_word(
            &mut game,
            ORACLE_TRIGONOMETRY_BASE + usize::from(primary_phase & SAMPLE_PHASE_MASK),
            ORACLE_PRIMARY_SAMPLE,
        );
        std::array::from_fn(|index| {
            let position = ORACLE_TRIGONOMETRY_BASE + index * 4;
            AlienTrigonometryPair {
                cosine: get_word(&game, position) as i16,
                sine: get_word(&game, position + ORACLE_WORD_BYTE_COUNT) as i16,
            }
        })
    }

    fn model_pose(local_position: [i32; AXIS_COUNT], counter: u16) -> AlienModelPose {
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes: vec![AlienNodePose {
                parent: AlienNodeParent::Root,
                first_vertex: usize::MIN,
                vertex_count: AUTHORED_VERTEX_COUNT,
                transform: AlienTransformData::default(),
                local_position,
                angles: [u16::MIN, counter, u16::MIN],
                radial_offset: i16::MIN,
            }],
            projected_vertices: vec![AlienProjectedVertex::default(); AUTHORED_VERTEX_COUNT],
            texture_coordinates: vec![[i16::MIN; 2]; AUTHORED_VERTEX_COUNT],
            object_positions: INITIAL_OBJECTS.to_vec(),
            authored_vertex_count: AUTHORED_VERTEX_COUNT,
            faces: Vec::<AlienFaceData>::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    fn object_image(case_index: usize, positions: &[[i16; AXIS_COUNT]]) -> Vec<u8> {
        let mut objects = patterned_image(
            case_index * ORACLE_OBJECT_CASE_STEP + ORACLE_OBJECT_BIAS,
            ORACLE_OBJECT_PATTERN_STRIDE,
        );
        for (object_index, position) in positions.iter().enumerate() {
            let base = ORACLE_OBJECT_BASE + object_index * ORACLE_OBJECT_BYTE_COUNT;
            put_word(
                &mut objects,
                base + ORACLE_DISTANCE_FIELD,
                position[X_AXIS] as u16,
            );
            put_word(
                &mut objects,
                base + ORACLE_MOTION_FIELD,
                position[Y_AXIS] as u16,
            );
            put_word(
                &mut objects,
                base + ORACLE_PHASE_FIELD,
                position[Z_AXIS] as u16,
            );
        }
        objects
    }

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn wave_initialization_matches_all_original_overlay_vectors() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_09ef_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0a30_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0a35_natural.json"),
        ];
        for fixture in fixtures {
            let vector: WaveVector = serde_json::from_value(
                serde_json::from_str::<Vec<serde_json::Value>>(fixture).unwrap()[0].clone(),
            )
            .unwrap();
            assert_eq!(vector.name, "initialize");
            let species = species(&vector.module);
            let mut pose = model_pose([i32::MIN; AXIS_COUNT], u16::MAX);
            let mut state = AlienWaveMethodState::default();
            let mut scene = AlienWaveSceneState {
                selection: AlienWaveSelection::Requested,
                selected_model: Some(PRESERVED_SELECTED_MODEL),
                current_sample: i16::MAX,
            };
            assert_eq!(
                update_or_initialize_wave(
                    species,
                    FIRST_NODE,
                    &mut pose,
                    &mut state,
                    &mut scene,
                    [ZERO_VIEW_COMPONENT; AXIS_COUNT],
                    &[AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT],
                )
                .unwrap(),
                AlienWaveUpdate::Initialized
            );
            assert_eq!(
                state,
                AlienWaveMethodState {
                    initialized: true,
                    primary_phase: INITIAL_PRIMARY_PHASE,
                    primary_step: INITIAL_PRIMARY_STEP,
                    secondary_phase: INITIAL_SECONDARY_PHASE,
                    secondary_step: INITIAL_SECONDARY_STEP,
                }
            );
            assert_eq!(scene.selection, AlienWaveSelection::Disabled);
            assert_eq!(pose.nodes[FIRST_NODE].radial_offset, INITIAL_RADIAL_OFFSET);
            assert_eq!(pose.nodes[FIRST_NODE].angles, [u16::MIN; AXIS_COUNT]);
            assert_eq!(
                scene.selected_model == Some(FIRST_NODE),
                vector.publishes_initial_state.unwrap()
            );
        }
    }

    #[test]
    fn wave_updates_match_every_original_overlay_vector_and_object_hash() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_09ef_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0a30_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0a35_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<WaveVector> = serde_json::from_str(fixture).unwrap();
            for (case_index, vector) in vectors.into_iter().skip(1).enumerate() {
                let case = UPDATE_CASES[case_index];
                let primary_phase = ORACLE_PRIMARY_PHASE_BASE
                    .wrapping_add(case_index as u16 * ORACLE_PRIMARY_PHASE_STEP);
                let mut state = AlienWaveMethodState {
                    initialized: true,
                    primary_phase,
                    primary_step: case.primary_step,
                    secondary_phase: ORACLE_SECONDARY_PHASE_BASE
                        .wrapping_add(case_index as u16 * ORACLE_SECONDARY_PHASE_STEP),
                    secondary_step: ORACLE_SECONDARY_STEP_BASE
                        .wrapping_add(case_index as i16 * ORACLE_SECONDARY_STEP_STEP),
                };
                let initial_counter = if case_index == usize::MIN {
                    u16::MAX
                } else {
                    case_index as u16
                };
                let mut pose = model_pose(case.local_position, initial_counter);
                let mut scene = AlienWaveSceneState {
                    selection: case.selection,
                    selected_model: Some(PRESERVED_SELECTED_MODEL),
                    current_sample: i16::MIN,
                };
                assert_eq!(
                    update_or_initialize_wave(
                        species(&vector.module),
                        case_index,
                        &mut pose,
                        &mut state,
                        &mut scene,
                        [ZERO_VIEW_COMPONENT; AXIS_COUNT],
                        &trigonometry(case_index, primary_phase),
                    )
                    .unwrap(),
                    AlienWaveUpdate::Advanced,
                    "{}",
                    vector.name
                );

                assert_eq!(
                    scene.selection,
                    selection(vector.selection_after.unwrap()),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.primary_step as u16,
                    vector.primary_step_after.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.primary_phase,
                    vector.primary_phase_after.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.secondary_phase,
                    vector.secondary_phase_after.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    scene.current_sample, ORACLE_CURRENT_SAMPLE,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    scene.selected_model,
                    if case.selected {
                        Some(case_index)
                    } else {
                        Some(PRESERVED_SELECTED_MODEL)
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].angles[SELECTION_COUNTER_AXIS],
                    initial_counter.wrapping_add(1),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    sha256(&object_image(case_index, &pose.object_positions)),
                    vector.object_data_sha256.unwrap(),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn active_wave_rejects_empty_or_truncated_typed_vertex_arrays() {
        let table = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
        let mut state = AlienWaveMethodState {
            initialized: true,
            ..AlienWaveMethodState::default()
        };
        let mut scene = AlienWaveSceneState::default();
        let mut pose = model_pose([i32::MIN; AXIS_COUNT], u16::MIN);
        pose.authored_vertex_count = usize::MIN;
        assert_eq!(
            update_or_initialize_wave(
                AlienSpecies::Amer,
                FIRST_NODE,
                &mut pose,
                &mut state,
                &mut scene,
                [ZERO_VIEW_COMPONENT; AXIS_COUNT],
                &table,
            ),
            Err(AlienWaveError::EmptyVertexList)
        );

        pose.authored_vertex_count = pose.object_positions.len() + 1;
        assert_eq!(
            update_or_initialize_wave(
                AlienSpecies::Amer,
                FIRST_NODE,
                &mut pose,
                &mut state,
                &mut scene,
                [ZERO_VIEW_COMPONENT; AXIS_COUNT],
                &table,
            ),
            Err(AlienWaveError::InvalidAuthoredVertexCount {
                authored: pose.object_positions.len() + 1,
                available: pose.object_positions.len(),
            })
        );
    }
}
