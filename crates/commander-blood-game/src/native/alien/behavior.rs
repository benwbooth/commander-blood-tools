//! Shared alien behavior operations expressed over typed model-node state.

use std::fmt;

use commander_blood_formats::alien::AXIS_COUNT;

use super::{AlienNodePose, AlienSpecies};

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

/// Invalid typed behavior state supplied to a recovered method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienBehaviorError {
    /// Every recovered behavior context contains at least one node.
    EmptyNodeList,
}

impl fmt::Display for AlienBehaviorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien behavior state: {self:?}")
    }
}

impl std::error::Error for AlienBehaviorError {}

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

fn high_word(value: i32) -> u16 {
    (value as u32 >> u16::BITS) as u16
}

fn within_signed_bounds(value: u16) -> bool {
    let value = value as i16;
    (-BOUNDS_LIMIT..=BOUNDS_LIMIT).contains(&value)
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienNodeParent, AlienTransformData};
    use serde::Deserialize;

    use super::*;

    const ZERO_COMPONENT: i32 = 0;

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
}
