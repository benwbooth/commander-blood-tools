//! Shared alien behavior operations expressed over typed model-node state.

use std::fmt;

use commander_blood_formats::alien::{AXIS_COUNT, AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT};

use super::{AlienModelPose, AlienNodePose, AlienSpecies};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const PRIMARY_NODE: usize = 0;
const BOUNDS_ANGLE_AXIS: usize = 1;
const STATE_ANGLE_AXIS: usize = 2;
const WRAP_BIAS: u16 = 0x4000;
const WRAP_MASK: u16 = 0x7fff;
const BOUNDS_ANGLE_STEP: u16 = 64;
const BOUNDS_LIMIT: i16 = 100;
const EXIT_REQUESTED: u16 = 1;
const STATE_ANGLE_DELTA: u16 = 15;
const METHOD_DELTA_SHIFT: u32 = 1;
const TEXTURE_U_AXIS: usize = 0;
const SCALED_SAMPLE_SHIFT: u32 = 4;
const SAMPLE_TABLE_STEP: usize = 1;

/// Invalid typed behavior state supplied to a recovered method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienBehaviorError {
    /// Every recovered behavior context contains at least one node.
    EmptyNodeList,
    /// A recovered sample method requires at least one authored vertex.
    EmptyVertexList,
    /// The model's authored vertex boundary exceeds its typed texture array.
    InvalidAuthoredVertexCount {
        /// Authored vertices requested by the model.
        authored: usize,
        /// Texture coordinates available in the runtime pose.
        available: usize,
    },
    /// A cyclic sample phase falls outside the decoded trigonometry table.
    InvalidSampleIndex {
        /// Invalid sample-table index.
        index: usize,
        /// Number of available samples.
        available: usize,
    },
}

impl fmt::Display for AlienBehaviorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien behavior state: {self:?}")
    }
}

impl std::error::Error for AlienBehaviorError {}

/// Typed continuation state for cyclic texture-coordinate animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienSampleState {
    /// Current entry in the decoded cosine table.
    pub table_index: usize,
    /// Sample published by the preceding update.
    pub previous: i16,
}

/// Wrap every node-local position around the camera's signed 15-bit world cell.
pub fn wrap_positions(
    nodes: &mut [AlienNodePose],
    view: [i16; AXIS_COUNT],
) -> Result<(), AlienBehaviorError> {
    if nodes.is_empty() {
        return Err(AlienBehaviorError::EmptyNodeList);
    }
    for node in nodes {
        for (position, origin) in node.local_position.iter_mut().zip(view) {
            let value = (*position as u16)
                .wrapping_add(origin as u16)
                .wrapping_add(WRAP_BIAS)
                & WRAP_MASK;
            *position = i32::from(value.wrapping_sub(WRAP_BIAS).wrapping_sub(origin as u16) as i16);
        }
    }
    Ok(())
}

/// Update the bounds accumulator, request scene exit when centered, then wrap.
pub fn bounds_then_wrap(
    nodes: &mut [AlienNodePose],
    view: [i16; AXIS_COUNT],
    exit_request: &mut u16,
) -> Result<(), AlienBehaviorError> {
    let primary = nodes
        .get_mut(PRIMARY_NODE)
        .ok_or(AlienBehaviorError::EmptyNodeList)?;
    primary.angles[BOUNDS_ANGLE_AXIS] =
        primary.angles[BOUNDS_ANGLE_AXIS].wrapping_add(BOUNDS_ANGLE_STEP);
    let bounds = primary.transform.translation.map(high_word);
    if bounds[Z_AXIS] <= BOUNDS_LIMIT as u16
        && within_signed_bounds(bounds[X_AXIS])
        && within_signed_bounds(bounds[Y_AXIS])
    {
        *exit_request = EXIT_REQUESTED;
    }
    wrap_positions(nodes, view)
}

/// Lower the first node's state angle and publish it as the active anchor.
pub fn anchor_state(nodes: &mut [AlienNodePose]) -> Result<usize, AlienBehaviorError> {
    let primary = nodes
        .get_mut(PRIMARY_NODE)
        .ok_or(AlienBehaviorError::EmptyNodeList)?;
    primary.angles[STATE_ANGLE_AXIS] =
        primary.angles[STATE_ANGLE_AXIS].wrapping_sub(STATE_ANGLE_DELTA);
    Ok(PRIMARY_NODE)
}

/// Apply the species-specific slot-twelve state-angle update.
pub fn adjust_state(
    species: AlienSpecies,
    nodes: &mut [AlienNodePose],
    method_delta: i16,
) -> Result<i16, AlienBehaviorError> {
    let primary = nodes
        .get_mut(PRIMARY_NODE)
        .ok_or(AlienBehaviorError::EmptyNodeList)?;
    if species == AlienSpecies::Scrut {
        primary.angles[STATE_ANGLE_AXIS] =
            primary.angles[STATE_ANGLE_AXIS].wrapping_sub(STATE_ANGLE_DELTA);
        return Ok(-(STATE_ANGLE_DELTA as i16));
    }

    let delta = method_delta >> METHOD_DELTA_SHIFT;
    if !delta.is_negative() {
        primary.angles[STATE_ANGLE_AXIS] =
            primary.angles[STATE_ANGLE_AXIS].wrapping_add(delta as u16);
    }
    Ok(delta)
}

/// Add one full-scale cyclic sample delta to every authored texture-U value.
pub fn apply_sample_delta(
    state: &mut AlienSampleState,
    pose: &mut AlienModelPose,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> Result<i16, AlienBehaviorError> {
    apply_sample_delta_with_scale(state, pose, trigonometry, false)
}

/// Add one sixteenth-scale cyclic sample delta to every authored texture-U value.
pub fn apply_scaled_sample_delta(
    state: &mut AlienSampleState,
    pose: &mut AlienModelPose,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> Result<i16, AlienBehaviorError> {
    apply_sample_delta_with_scale(state, pose, trigonometry, true)
}

fn apply_sample_delta_with_scale(
    state: &mut AlienSampleState,
    pose: &mut AlienModelPose,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    scaled: bool,
) -> Result<i16, AlienBehaviorError> {
    if pose.authored_vertex_count == usize::MIN {
        return Err(AlienBehaviorError::EmptyVertexList);
    }
    if pose.authored_vertex_count > pose.texture_coordinates.len() {
        return Err(AlienBehaviorError::InvalidAuthoredVertexCount {
            authored: pose.authored_vertex_count,
            available: pose.texture_coordinates.len(),
        });
    }
    let sample =
        trigonometry
            .get(state.table_index)
            .ok_or(AlienBehaviorError::InvalidSampleIndex {
                index: state.table_index,
                available: trigonometry.len(),
            })?;
    let current = if scaled {
        sample.cosine >> SCALED_SAMPLE_SHIFT
    } else {
        sample.cosine
    };
    let delta = current.wrapping_sub(state.previous);
    state.table_index = (state.table_index + SAMPLE_TABLE_STEP) % TRIGONOMETRY_ENTRY_COUNT;
    state.previous = current;
    for texture in &mut pose.texture_coordinates[..pose.authored_vertex_count] {
        texture[TEXTURE_U_AXIS] = texture[TEXTURE_U_AXIS].wrapping_add(delta);
    }
    Ok(delta)
}

fn high_word(value: i32) -> u16 {
    (value as u32 >> u16::BITS) as u16
}

fn within_signed_bounds(value: u16) -> bool {
    let value = value as i16;
    (-BOUNDS_LIMIT..=BOUNDS_LIMIT).contains(&value)
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienFaceData, AlienNodeParent, AlienTransformData};
    use serde::Deserialize;

    use super::*;

    const ZERO_COMPONENT: i32 = 0;
    const SAMPLE_RECORD_BYTE_COUNT: u16 = 4;
    const OBJECT_RECORD_BYTE_COUNT: u16 = 20;
    const BYTE_PATTERN_MULTIPLIER: u32 = 37;
    const CASE_PATTERN_STEP: u32 = 11;
    const BYTE_MASK: u32 = 0xff;
    const HIGH_BYTE_SHIFT: u32 = 8;
    const TEXTURE_V_SENTINEL: i16 = 123;

    #[derive(Deserialize)]
    struct WrapStateVector {
        input_low_words: [u16; AXIS_COUNT],
        output_dwords: [u32; AXIS_COUNT],
    }

    #[derive(Deserialize)]
    struct WrapVector {
        name: String,
        state_count: u16,
        view_origin_words: Option<[u16; AXIS_COUNT]>,
        states: Option<Vec<WrapStateVector>>,
    }

    #[derive(Deserialize)]
    struct BoundsVector {
        name: String,
        bounds: BoundsValues,
        accumulator_before: u16,
        accumulator_after: u16,
        exit_before: u16,
        exit_after: u16,
    }

    #[derive(Deserialize)]
    struct BoundsValues {
        first_signed: i16,
        second_signed: i16,
        unsigned_axis: u16,
    }

    #[derive(Deserialize)]
    struct AngleVector {
        name: String,
        field_before: u16,
        field_after: u16,
    }

    #[derive(Deserialize)]
    struct DeltaVector {
        name: String,
        delta_before: u16,
        half_delta: u16,
        field_before: u16,
        field_after: u16,
    }

    #[derive(Deserialize)]
    struct SampleVector {
        name: String,
        scaled: bool,
        sample_cursor_before: u16,
        sample_cursor_after: u16,
        raw_sample: u16,
        current_sample: u16,
        previous_sample: u16,
        delta: u16,
        object_offset: u16,
        object_count: u16,
        effective_iterations: usize,
    }

    fn node() -> AlienNodePose {
        AlienNodePose {
            parent: AlienNodeParent::Root,
            first_vertex: usize::MIN,
            vertex_count: 1,
            transform: AlienTransformData::default(),
            local_position: [ZERO_COMPONENT; AXIS_COUNT],
            angles: [u16::MIN; AXIS_COUNT],
            radial_offset: i16::MIN,
        }
    }

    fn translation_with_high_words(words: [u16; AXIS_COUNT]) -> [i32; AXIS_COUNT] {
        words.map(|word| (u32::from(word) << u16::BITS) as i32)
    }

    fn sample_pose(texture_coordinates: Vec<[i16; 2]>) -> AlienModelPose {
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes: Vec::new(),
            projected_vertices: vec![Default::default(); texture_coordinates.len()],
            authored_vertex_count: texture_coordinates.len(),
            object_positions: vec![[i16::MIN; AXIS_COUNT]; texture_coordinates.len()],
            texture_coordinates,
            faces: Vec::<AlienFaceData>::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    fn patterned_object_word(case_index: usize, object_position: u16) -> i16 {
        let byte = |position: u16| {
            ((u32::from(position) * BYTE_PATTERN_MULTIPLIER
                + case_index as u32 * CASE_PATTERN_STEP)
                & BYTE_MASK) as u8
        };
        let low = u16::from(byte(object_position));
        let high = u16::from(byte(object_position.wrapping_add(1)));
        (low | (high << HIGH_BYTE_SHIFT)) as i16
    }

    #[test]
    fn position_wrapping_matches_all_typed_original_overlay_vectors() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0958_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0999_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0999_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<WrapVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                if vector.state_count == u16::MIN {
                    assert_eq!(
                        wrap_positions(&mut [], [i16::MIN; AXIS_COUNT]),
                        Err(AlienBehaviorError::EmptyNodeList),
                        "{}",
                        vector.name
                    );
                    continue;
                }
                let states = vector.states.unwrap();
                assert_eq!(states.len(), usize::from(vector.state_count));
                let mut nodes = states
                    .iter()
                    .map(|state| {
                        let mut node = node();
                        node.local_position =
                            state.input_low_words.map(|word| i32::from(word as i16));
                        node
                    })
                    .collect::<Vec<_>>();
                wrap_positions(
                    &mut nodes,
                    vector.view_origin_words.unwrap().map(|word| word as i16),
                )
                .unwrap();
                for (node, expected) in nodes.iter().zip(states) {
                    assert_eq!(
                        node.local_position.map(|value| value as u32),
                        expected.output_dwords,
                        "{}",
                        vector.name
                    );
                }
            }
        }
    }

    #[test]
    fn bounds_and_exit_updates_match_every_original_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0925_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0966_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0966_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<BoundsVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut primary = node();
                primary.transform.translation = translation_with_high_words([
                    vector.bounds.first_signed as u16,
                    vector.bounds.second_signed as u16,
                    vector.bounds.unsigned_axis,
                ]);
                primary.angles[BOUNDS_ANGLE_AXIS] = vector.accumulator_before;
                let mut nodes = [primary];
                let mut exit_request = vector.exit_before;
                bounds_then_wrap(&mut nodes, [i16::MIN; AXIS_COUNT], &mut exit_request).unwrap();
                assert_eq!(
                    nodes[PRIMARY_NODE].angles[BOUNDS_ANGLE_AXIS], vector.accumulator_after,
                    "{}",
                    vector.name
                );
                assert_eq!(exit_request, vector.exit_after, "{}", vector.name);
            }
        }
    }

    #[test]
    fn anchor_updates_match_every_original_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0b0f_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0b50_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0b55_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<AngleVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut primary = node();
                primary.angles[STATE_ANGLE_AXIS] = vector.field_before;
                let mut nodes = [primary];
                assert_eq!(anchor_state(&mut nodes).unwrap(), PRIMARY_NODE);
                assert_eq!(
                    nodes[PRIMARY_NODE].angles[STATE_ANGLE_AXIS], vector.field_after,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn species_adjustments_match_every_original_overlay_vector() {
        let shared_fixtures = [
            (
                AlienSpecies::Amer,
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_amer_func_0b1f_natural.json"
                ),
            ),
            (
                AlienSpecies::Croolis,
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0b60_natural.json"
                ),
            ),
        ];
        for (species, fixture) in shared_fixtures {
            let vectors: Vec<DeltaVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut primary = node();
                primary.angles[STATE_ANGLE_AXIS] = vector.field_before;
                let mut nodes = [primary];
                assert_eq!(
                    adjust_state(species, &mut nodes, vector.delta_before as i16).unwrap() as u16,
                    vector.half_delta,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    nodes[PRIMARY_NODE].angles[STATE_ANGLE_AXIS], vector.field_after,
                    "{}",
                    vector.name
                );
            }
        }

        let vectors: Vec<AngleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_scrut_func_0b65_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut primary = node();
            primary.angles[STATE_ANGLE_AXIS] = vector.field_before;
            let mut nodes = [primary];
            assert_eq!(
                adjust_state(AlienSpecies::Scrut, &mut nodes, i16::MAX).unwrap(),
                -(STATE_ANGLE_DELTA as i16),
                "{}",
                vector.name
            );
            assert_eq!(
                nodes[PRIMARY_NODE].angles[STATE_ANGLE_AXIS], vector.field_after,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn sample_driven_texture_updates_match_every_typed_original_overlay_vector() {
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1b5f_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1acb_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1b80_natural.json"),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1b8f_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1afb_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1bb0_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<SampleVector> = serde_json::from_str(fixture).unwrap();
            for (case_index, vector) in vectors.into_iter().enumerate() {
                if vector.sample_cursor_before % SAMPLE_RECORD_BYTE_COUNT != u16::MIN {
                    assert_eq!(vector.name, "odd_cursor");
                    continue;
                }
                let table_index =
                    usize::from(vector.sample_cursor_before / SAMPLE_RECORD_BYTE_COUNT);
                let mut state = AlienSampleState {
                    table_index,
                    previous: vector.previous_sample as i16,
                };
                let mut table = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                table[table_index].cosine = vector.raw_sample as i16;
                if vector.object_count == u16::MIN {
                    assert!(vector.effective_iterations > usize::MIN);
                    let mut pose = sample_pose(Vec::new());
                    let result = if vector.scaled {
                        apply_scaled_sample_delta(&mut state, &mut pose, &table)
                    } else {
                        apply_sample_delta(&mut state, &mut pose, &table)
                    };
                    assert_eq!(result, Err(AlienBehaviorError::EmptyVertexList));
                    continue;
                }

                assert_eq!(
                    vector.effective_iterations,
                    usize::from(vector.object_count)
                );
                let original_u = (0..vector.object_count)
                    .map(|index| {
                        let position = vector
                            .object_offset
                            .wrapping_add(index.wrapping_mul(OBJECT_RECORD_BYTE_COUNT));
                        patterned_object_word(case_index, position)
                    })
                    .collect::<Vec<_>>();
                let mut pose = sample_pose(
                    original_u
                        .iter()
                        .map(|coordinate| [*coordinate, TEXTURE_V_SENTINEL])
                        .collect(),
                );
                let delta = if vector.scaled {
                    apply_scaled_sample_delta(&mut state, &mut pose, &table)
                } else {
                    apply_sample_delta(&mut state, &mut pose, &table)
                }
                .unwrap();

                assert_eq!(delta as u16, vector.delta, "{}", vector.name);
                assert_eq!(
                    state.table_index,
                    usize::from(vector.sample_cursor_after / SAMPLE_RECORD_BYTE_COUNT),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.previous as u16, vector.current_sample,
                    "{}",
                    vector.name
                );
                for (texture, original) in pose.texture_coordinates.iter().zip(original_u) {
                    assert_eq!(
                        texture[TEXTURE_U_AXIS],
                        original.wrapping_add(vector.delta as i16),
                        "{}",
                        vector.name
                    );
                    assert_eq!(texture[1], TEXTURE_V_SENTINEL, "{}", vector.name);
                }
            }
        }
    }

    #[test]
    fn sample_updates_validate_flat_typed_ranges_before_mutating_state() {
        let table = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
        let mut state = AlienSampleState {
            table_index: TRIGONOMETRY_ENTRY_COUNT,
            previous: i16::MAX,
        };
        let mut pose = sample_pose(vec![[i16::MIN; 2]]);
        assert_eq!(
            apply_sample_delta(&mut state, &mut pose, &table),
            Err(AlienBehaviorError::InvalidSampleIndex {
                index: TRIGONOMETRY_ENTRY_COUNT,
                available: TRIGONOMETRY_ENTRY_COUNT,
            })
        );

        state.table_index = usize::MIN;
        pose.authored_vertex_count = pose.texture_coordinates.len() + 1;
        assert_eq!(
            apply_sample_delta(&mut state, &mut pose, &table),
            Err(AlienBehaviorError::InvalidAuthoredVertexCount {
                authored: pose.texture_coordinates.len() + 1,
                available: pose.texture_coordinates.len(),
            })
        );
    }
}
