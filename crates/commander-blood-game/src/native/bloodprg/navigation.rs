//! Typed object relationships used by BloodScript ship-navigation handlers.

use std::collections::BTreeSet;
use std::fmt;

use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference, ScriptStateWord,
    ScriptStateWordPair,
};

use crate::native::math::binary_u32_sqrt;

use super::{
    ScriptFieldSelector, ScriptObjectFlag, active_objects_in_play, object_has_flag,
    script_field_offset,
};

const BITS_PER_BYTE: usize = u8::BITS as usize;
const WORD_SIGN_BIT: u16 = 1_u16 << (u16::BITS - 1);

/// Failure while traversing typed profile-object relationships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptNavigationError {
    /// A known object kind's parent field is outside its decoded record.
    MissingParentField {
        /// Object containing the invalid field.
        object: ScriptObjectId,
    },
    /// A parent field contains neither a decoded object nor the native sentinel.
    InvalidParentReference {
        /// Object containing the invalid relation.
        object: ScriptObjectId,
    },
    /// Parent relations contain a cycle instead of a navigation tree.
    CyclicParentRelations {
        /// Object encountered twice on one recursion path.
        object: ScriptObjectId,
    },
    /// A requested object identity is absent from this decoded profile.
    MissingObject {
        /// Missing object identity.
        object: ScriptObjectId,
    },
    /// A known object kind has no bounded coordinate pair for this path.
    MissingPositionField {
        /// Object whose coordinates could not be resolved.
        object: ScriptObjectId,
    },
    /// A black-hole selector or relation word is absent from its typed record.
    MissingPositionSelector {
        /// Object containing the missing selector field.
        object: ScriptObjectId,
    },
}

impl fmt::Display for ScriptNavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptNavigationError {}

/// Test whether one character's recovered link set contains another object.
///
/// This translates `ship_3d_object_table_bit_test_full` at BLOODPRG file
/// offset `0x006210`. Directory positions become stable object indices and the
/// selected byte remains bounded by the source object's decoded record.
pub fn object_links_to(
    state: &ScriptState,
    source: ScriptObjectId,
    target: ScriptObjectId,
) -> Option<bool> {
    let source_object = state.object(source)?;
    let field_offset = script_field_offset(source_object.kind, ScriptFieldSelector::OBJECT_LINKS)?;
    let byte_index = field_offset.checked_add(target.index() / BITS_PER_BYTE)?;
    let byte = state.byte(state.object_byte(source, byte_index)?)?;
    let bit_index = target.index() % BITS_PER_BYTE;
    let mask = 1_u8 << (BITS_PER_BYTE - 1 - bit_index);
    Some(byte & mask != u8::MIN)
}

/// Return all descendants whose parent relation reaches `target`, depth first.
///
/// This translates `ship_3d_nav_source_list_build_full` at BLOODPRG file
/// offset `0x00624B`. The returned vector replaces the original scratch-buffer
/// offsets and terminal `0xFFFF` word with typed object identities and length.
pub fn navigation_source_objects(
    state: &ScriptState,
    target: ScriptObjectId,
) -> Result<Vec<ScriptObjectId>, ScriptNavigationError> {
    let mut output = Vec::new();
    append_navigation_children(state, target, &mut BTreeSet::new(), &mut output)?;
    Ok(output)
}

/// Build the active actor choices reachable below one navigation target.
///
/// This translates `ship_3d_navigation_candidate_build` at BLOODPRG file
/// offset `0x0070EE`. The source traversal runs before filtering, preserving
/// authored depth-first order. Owned object identities replace both native
/// scratch buffers and their incompatible terminators.
pub fn navigation_candidates(
    state: &ScriptState,
    target: ScriptObjectId,
    honk: ScriptObjectId,
) -> Result<Vec<ScriptObjectId>, ScriptNavigationError> {
    let source = navigation_source_objects(state, target)?;
    Ok(filter_navigation_candidates(state, &source, honk))
}

/// Return active navigation objects sharing Arche's current position.
///
/// This translates `vm_state_record_processor` at BLOODPRG file offset
/// `0x00713D`. Active decoded objects replace the native directory walk;
/// location coordinates resolve through their typed holder relation, and the
/// provisional offset buffer becomes a collected sequence of object IDs.
pub fn objects_at_arche_position(
    state: &ScriptState,
    arche: ScriptObjectId,
) -> Result<Vec<ScriptObjectId>, ScriptNavigationError> {
    let candidates = state
        .objects()
        .iter()
        .map(|object| object.id)
        .collect::<Vec<_>>();
    filter_objects_at_arche_position(state, arche, &candidates)
}

/// Return in-play actors offered by the navigation-choice interface.
///
/// This translates `nav_kind2_target_list_build` at BLOODPRG file offset
/// `0x0071CF`. The in-play helper supplies owned IDs, while Honk and Radio are
/// explicit typed exclusions rather than offsets in a terminated scratch list.
pub fn navigation_actor_targets(
    state: &ScriptState,
    honk: ScriptObjectId,
    radio: ScriptObjectId,
) -> Vec<ScriptObjectId> {
    let source = active_objects_in_play(state);
    filter_navigation_actor_targets(state, &source, honk, radio)
}

/// Return in-play objects displayed by the navigation chart.
///
/// This translates `nav_chart_list_build` at BLOODPRG file offset `0x00721A`.
/// Length-delimited object IDs replace the native signed terminator while
/// preserving the helper-first order and exact supported object classes.
pub fn navigation_chart_objects(state: &ScriptState) -> Vec<ScriptObjectId> {
    let source = active_objects_in_play(state);
    filter_navigation_chart_objects(state, &source)
}

/// Resolve the live coordinate pair used for one navigation object.
///
/// This translates `ship_3d_position_field_resolve` at BLOODPRG file offset
/// `0x0061A6`. Parent links and the arche fallback are typed identities, and a
/// cycle or unsupported record shape is rejected instead of dereferencing an
/// arbitrary word as an address.
pub fn resolve_navigation_position(
    state: &ScriptState,
    object: ScriptObjectId,
    arche: ScriptObjectId,
    black_hole_compare: u16,
) -> Result<ScriptStateWordPair, ScriptNavigationError> {
    let mut current = object;
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current) {
            return Err(ScriptNavigationError::CyclicParentRelations { object: current });
        }
        let state_object = state
            .object(current)
            .ok_or(ScriptNavigationError::MissingObject { object: current })?;
        match state_object.kind {
            ScriptObjectKind::BlackHole => {
                let comparison = read_object_word(
                    state,
                    current,
                    ScriptFieldSelector::BLACK_HOLE_COMPARISON,
                )?;
                let selector = if comparison == black_hole_compare {
                    ScriptFieldSelector::BLACK_HOLE_MATCH_POSITION
                } else {
                    ScriptFieldSelector::BLACK_HOLE_MISMATCH_POSITION
                };
                return object_word_pair(state, current, selector);
            }
            ScriptObjectKind::CelestialBody
            | ScriptObjectKind::NavigationEntity
            | ScriptObjectKind::WorldState => {
                return object_word_pair(
                    state,
                    current,
                    ScriptFieldSelector::NAVIGATION_POSITION,
                );
            }
            _ => {
                let parent = object_reference(
                    state,
                    current,
                    ScriptFieldSelector::HOLDER_OR_LOCATION,
                )?;
                current = match parent {
                    ScriptStateObjectReference::Object(parent) => parent,
                    ScriptStateObjectReference::Sentinel => arche,
                };
            }
        }
    }
}

/// Calculate the original wrapped two-dimensional navigation distance.
///
/// This translates `ship_3d_position_distance` at BLOODPRG file offset
/// `0x0060DD`, including contextual black-hole pair selection, the auxiliary
/// record's direct first pair, wrapped signed deltas, and the recovered native
/// integer square root.
pub fn navigation_distance(
    state: &ScriptState,
    first: ScriptObjectId,
    second: ScriptObjectId,
    arche: ScriptObjectId,
    inherited_black_hole_compare: u16,
) -> Result<u16, ScriptNavigationError> {
    let mut compare = inherited_black_hole_compare;
    let first_kind = state
        .object(first)
        .ok_or(ScriptNavigationError::MissingObject { object: first })?
        .kind;
    let second_kind = state
        .object(second)
        .ok_or(ScriptNavigationError::MissingObject { object: second })?
        .kind;

    let first_position = if first_kind == ScriptObjectKind::BlackHole {
        compare = read_object_word(
            state,
            second,
            ScriptFieldSelector::BLACK_HOLE_RELATION,
        )?;
        resolve_navigation_position(state, first, arche, compare)?
    } else if first_kind == ScriptObjectKind::Auxiliary {
        state
            .object_word_pair(first, usize::MIN)
            .ok_or(ScriptNavigationError::MissingPositionField { object: first })?
    } else {
        resolve_navigation_position(state, first, arche, compare)?
    };

    let second_position = if second_kind == ScriptObjectKind::BlackHole {
        compare = read_object_word(
            state,
            first,
            ScriptFieldSelector::BLACK_HOLE_RELATION,
        )?;
        resolve_navigation_position(state, second, arche, compare)?
    } else if second_kind == ScriptObjectKind::Auxiliary {
        state
            .object_word_pair(second, usize::MIN)
            .ok_or(ScriptNavigationError::MissingPositionField { object: second })?
    } else {
        resolve_navigation_position(state, second, arche, compare)?
    };

    let first_position = state
        .word_pair(first_position)
        .ok_or(ScriptNavigationError::MissingPositionField { object: first })?;
    let second_position = state
        .word_pair(second_position)
        .ok_or(ScriptNavigationError::MissingPositionField { object: second })?;
    Ok(distance_between_positions(first_position, second_position))
}

fn append_navigation_children(
    state: &ScriptState,
    target: ScriptObjectId,
    path: &mut BTreeSet<ScriptObjectId>,
    output: &mut Vec<ScriptObjectId>,
) -> Result<(), ScriptNavigationError> {
    if !path.insert(target) {
        return Err(ScriptNavigationError::CyclicParentRelations { object: target });
    }

    for object in state.objects() {
        let Some(field_offset) =
            script_field_offset(object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
        else {
            continue;
        };
        let field = state
            .object_word(object.id, field_offset / std::mem::size_of::<u16>())
            .ok_or(ScriptNavigationError::MissingParentField { object: object.id })?;
        let parent = state
            .object_reference(field)
            .ok_or(ScriptNavigationError::InvalidParentReference { object: object.id })?;
        if parent != ScriptStateObjectReference::Object(target) {
            continue;
        }
        output.push(object.id);
        append_navigation_children(state, object.id, path, output)?;
    }

    path.remove(&target);
    Ok(())
}

fn filter_navigation_candidates(
    state: &ScriptState,
    source: &[ScriptObjectId],
    honk: ScriptObjectId,
) -> Vec<ScriptObjectId> {
    source
        .iter()
        .copied()
        .filter(|object| *object != honk)
        .filter(|object| {
            state.object(*object).is_some_and(|record| {
                record.kind == ScriptObjectKind::Actor
                    && object_has_flag(state, *object, ScriptObjectFlag::Active) == Some(true)
            })
        })
        .collect()
}

fn filter_objects_at_arche_position(
    state: &ScriptState,
    arche: ScriptObjectId,
    candidates: &[ScriptObjectId],
) -> Result<Vec<ScriptObjectId>, ScriptNavigationError> {
    let arche_position = object_word_pair(
        state,
        arche,
        ScriptFieldSelector::NAVIGATION_POSITION,
    )?;
    let arche_position = state
        .word_pair(arche_position)
        .ok_or(ScriptNavigationError::MissingPositionField { object: arche })?;
    let mut output = Vec::new();

    for candidate in candidates.iter().copied() {
        let candidate_record = state
            .object(candidate)
            .ok_or(ScriptNavigationError::MissingObject { object: candidate })?;
        if object_has_flag(state, candidate, ScriptObjectFlag::Active) != Some(true)
            || candidate == arche
        {
            continue;
        }

        let effective = match candidate_record.kind {
            ScriptObjectKind::CelestialBody | ScriptObjectKind::NavigationEntity => candidate,
            ScriptObjectKind::Location => {
                let linked = match object_reference(
                    state,
                    candidate,
                    ScriptFieldSelector::HOLDER_OR_LOCATION,
                )? {
                    ScriptStateObjectReference::Object(linked) => linked,
                    ScriptStateObjectReference::Sentinel => {
                        return Err(ScriptNavigationError::InvalidParentReference {
                            object: candidate,
                        });
                    }
                };
                let linked_record = state
                    .object(linked)
                    .ok_or(ScriptNavigationError::MissingObject { object: linked })?;
                if object_has_flag(state, linked, ScriptObjectFlag::Active) != Some(true)
                    || !matches!(
                        linked_record.kind,
                        ScriptObjectKind::CelestialBody | ScriptObjectKind::NavigationEntity
                    )
                {
                    continue;
                }
                linked
            }
            _ => continue,
        };

        let position = object_word_pair(
            state,
            effective,
            ScriptFieldSelector::NAVIGATION_POSITION,
        )?;
        if state.word_pair(position) == Some(arche_position) {
            output.push(candidate);
        }
    }

    Ok(output)
}

fn filter_navigation_actor_targets(
    state: &ScriptState,
    source: &[ScriptObjectId],
    honk: ScriptObjectId,
    radio: ScriptObjectId,
) -> Vec<ScriptObjectId> {
    source
        .iter()
        .copied()
        .filter(|object| *object != honk && *object != radio)
        .filter(|object| {
            state
                .object(*object)
                .is_some_and(|record| record.kind == ScriptObjectKind::Actor)
        })
        .collect()
}

fn filter_navigation_chart_objects(
    state: &ScriptState,
    source: &[ScriptObjectId],
) -> Vec<ScriptObjectId> {
    source
        .iter()
        .copied()
        .filter(|object| {
            state.object(*object).is_some_and(|record| {
                matches!(
                    record.kind,
                    ScriptObjectKind::CelestialBody
                        | ScriptObjectKind::NavigationEntity
                        | ScriptObjectKind::BlackHole
                )
            })
        })
        .collect()
}

fn object_field(
    state: &ScriptState,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<ScriptStateWord, ScriptNavigationError> {
    let state_object = state
        .object(object)
        .ok_or(ScriptNavigationError::MissingObject { object })?;
    let byte_offset = script_field_offset(state_object.kind, selector)
        .ok_or(ScriptNavigationError::MissingPositionSelector { object })?;
    state
        .object_word(object, byte_offset / std::mem::size_of::<u16>())
        .ok_or(ScriptNavigationError::MissingPositionSelector { object })
}

fn read_object_word(
    state: &ScriptState,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<u16, ScriptNavigationError> {
    state
        .word(object_field(state, object, selector)?)
        .ok_or(ScriptNavigationError::MissingPositionSelector { object })
}

fn object_word_pair(
    state: &ScriptState,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<ScriptStateWordPair, ScriptNavigationError> {
    let field = object_field(state, object, selector)?;
    state
        .object_word_pair(object, field.word_index())
        .ok_or(ScriptNavigationError::MissingPositionField { object })
}

fn object_reference(
    state: &ScriptState,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<ScriptStateObjectReference, ScriptNavigationError> {
    let field = object_field(state, object, selector)
        .map_err(|_| ScriptNavigationError::MissingParentField { object })?;
    state
        .object_reference(field)
        .ok_or(ScriptNavigationError::InvalidParentReference { object })
}

fn distance_between_positions(first: [u16; 2], second: [u16; 2]) -> u16 {
    binary_u32_sqrt(squared_distance_between_positions(first, second))
}

fn squared_distance_between_positions(first: [u16; 2], second: [u16; 2]) -> u32 {
    let dx = wrapped_absolute_delta(first[0], second[0]);
    let dy = wrapped_absolute_delta(first[1], second[1]);
    (dx * dx) as u32 + (dy * dy) as u32
}

fn wrapped_absolute_delta(first: u16, second: u16) -> i32 {
    let delta = first.wrapping_sub(second);
    let absolute = if delta & WORD_SIGN_BIT != u16::MIN {
        u16::MIN.wrapping_sub(delta)
    } else {
        delta
    };
    i32::from(absolute as i16)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::script::{
        ScriptObjectKind, decode_script_directory, decode_script_state,
    };
    use serde::Deserialize;

    use super::*;

    const OBJECT_LINK_VECTOR_COUNT: usize = 8;
    const NAVIGATION_SOURCE_VECTOR_COUNT: usize = 8;
    const NAVIGATION_CANDIDATE_VECTOR_COUNT: usize = 7;
    const ARCHE_POSITION_VECTOR_COUNT: usize = 16;
    const NAVIGATION_ACTOR_TARGET_VECTOR_COUNT: usize = 9;
    const NAVIGATION_CHART_VECTOR_COUNT: usize = 7;
    const POSITION_RESOLVER_VECTOR_COUNT: usize = 8;
    const POSITION_DISTANCE_VECTOR_COUNT: usize = 6;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const OBJECT_FLAGS_BYTE_OFFSET: usize = 2;
    const MATCHING_POSITION: [u16; 2] = [1_200, 3_400];
    const MISMATCHING_POSITION: [u16; 2] = [5_600, 7_800];

    #[derive(Deserialize)]
    struct ObjectLinkOracle {
        name: String,
        directory_index: usize,
        bitset_byte: u8,
        defined_flags: ObjectLinkFlags,
    }

    #[derive(Deserialize)]
    struct ObjectLinkFlags {
        carry: u8,
    }

    #[derive(Deserialize)]
    struct NavigationSourceOracle {
        name: String,
        output_offsets: Vec<u16>,
    }

    #[derive(Deserialize)]
    struct NavigationCandidateOracle {
        name: String,
        source: Vec<u16>,
        output: Vec<u16>,
    }

    #[derive(Deserialize)]
    struct ArchePositionOracle {
        name: String,
        processed_entries: Vec<u16>,
        output: Vec<u16>,
    }

    #[derive(Deserialize)]
    struct NavigationActorTargetOracle {
        name: String,
        source: Vec<u16>,
        output: Vec<u16>,
        count: usize,
    }

    #[derive(Deserialize)]
    struct NavigationChartOracle {
        name: String,
        active_object_offsets: Vec<u16>,
        nav_chart_offsets: Vec<u16>,
        count: usize,
    }

    struct ArchePositionCase {
        kinds: Vec<ScriptObjectKind>,
        parents: Vec<Option<usize>>,
        flags: Vec<u8>,
        positions: Vec<(usize, [u16; 2])>,
        source_indices: Vec<usize>,
    }

    struct NavigationActorTargetCase {
        kinds: Vec<ScriptObjectKind>,
        source_indices: Vec<usize>,
        honk_index: usize,
        radio_index: usize,
    }

    struct NavigationChartCase {
        kinds: Vec<ScriptObjectKind>,
        source_indices: Vec<usize>,
    }

    #[derive(Deserialize)]
    struct PositionResolverOracle {
        name: String,
    }

    #[derive(Deserialize)]
    struct PositionDistanceOracle {
        name: String,
        squared_distance: u32,
        eax: u32,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn object_links_match_every_original_handler_vector() {
        let vectors: Vec<ObjectLinkOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6210_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), OBJECT_LINK_VECTOR_COUNT);
        let directory =
            decode_script_directory(&std::fs::read(original_asset("SCRIPT1.DEB")).unwrap())
                .unwrap();
        let original_state = decode_script_state(
            &std::fs::read(original_asset("SCRIPT1.VAR")).unwrap(),
            &directory,
        )
        .unwrap();
        let source = original_state
            .objects()
            .iter()
            .find(|object| object.kind == ScriptObjectKind::Actor)
            .unwrap()
            .id;
        let links_offset =
            script_field_offset(ScriptObjectKind::Actor, ScriptFieldSelector::OBJECT_LINKS)
                .unwrap();

        for vector in vectors {
            let mut state = original_state.clone();
            let target = state.objects()[vector.directory_index].id;
            let byte_index = links_offset + vector.directory_index / BITS_PER_BYTE;
            let byte = state.object_byte(source, byte_index).unwrap();
            assert!(state.set_byte(byte, vector.bitset_byte));
            assert_eq!(
                object_links_to(&state, source, target),
                Some(vector.defined_flags.carry != u8::MIN),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn navigation_sources_match_every_original_depth_first_case() {
        let vectors: Vec<NavigationSourceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_624b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), NAVIGATION_SOURCE_VECTOR_COUNT);

        for vector in vectors {
            let (kinds, parents, expected_indices): (&[_], &[_], &[_]) = match vector.name.as_str()
            {
                "no_children" => (
                    &[ScriptObjectKind::Actor, ScriptObjectKind::Actor],
                    &[None, None],
                    &[],
                ),
                "one_child"
                | "next_inactive_entry_stops_scan"
                | "directory_and_object_field_wrap" => (
                    &[ScriptObjectKind::Actor, ScriptObjectKind::Actor],
                    &[None, Some(0)],
                    &[1],
                ),
                "two_siblings" | "output_cursor_wrap" => (
                    &[
                        ScriptObjectKind::Actor,
                        ScriptObjectKind::Actor,
                        ScriptObjectKind::Location,
                    ],
                    &[None, Some(0), Some(0)],
                    &[1, 2],
                ),
                "depth_first_child_before_sibling" => (
                    &[
                        ScriptObjectKind::Actor,
                        ScriptObjectKind::Actor,
                        ScriptObjectKind::Location,
                        ScriptObjectKind::Actor,
                    ],
                    &[None, Some(0), Some(1), Some(0)],
                    &[1, 2, 3],
                ),
                "zero_field_offset_is_skipped" => (
                    &[
                        ScriptObjectKind::Actor,
                        ScriptObjectKind::Auxiliary,
                        ScriptObjectKind::Actor,
                    ],
                    &[None, None, Some(0)],
                    &[2],
                ),
                name => panic!("unknown navigation-source oracle {name}"),
            };
            assert_eq!(
                expected_indices.len(),
                vector.output_offsets.len(),
                "{}",
                vector.name
            );
            let state = navigation_fixture(kinds, parents);
            let expected = expected_indices
                .iter()
                .map(|index| state.objects()[*index].id)
                .collect::<Vec<_>>();

            assert_eq!(
                navigation_source_objects(&state, state.objects()[0].id).unwrap(),
                expected,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn navigation_candidate_filter_matches_every_original_case() {
        let vectors: Vec<NavigationCandidateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_70ee_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), NAVIGATION_CANDIDATE_VECTOR_COUNT);

        for vector in vectors {
            let (kinds, flags, source_indices, honk_index): (&[_], &[_], &[_], usize) =
                match vector.name.as_str() {
                    "empty_source" => (&[ScriptObjectKind::Actor], &[1], &[], 0),
                    "two_active_kind_two" => (
                        &[ScriptObjectKind::Actor; 3],
                        &[0, 1, 165],
                        &[1, 2],
                        0,
                    ),
                    "exclude_honk_before_record_read" => {
                        (&[ScriptObjectKind::Actor; 3], &[0, 1, 1], &[1, 2], 1)
                    }
                    "mixed_kind_and_activity" => (
                        &[
                            ScriptObjectKind::Actor,
                            ScriptObjectKind::Player,
                            ScriptObjectKind::Actor,
                            ScriptObjectKind::BlackHole,
                            ScriptObjectKind::Actor,
                            ScriptObjectKind::Actor,
                        ],
                        &[0, 1, 0, 1, 128, 129],
                        &[1, 2, 3, 4, 5],
                        0,
                    ),
                    "zero_offset_is_valid" | "unsigned_high_offsets" => (
                        &[ScriptObjectKind::Actor; 3],
                        &[1, 1, 0],
                        &[0, 1],
                        2,
                    ),
                    "all_rejected" => (
                        &[
                            ScriptObjectKind::Actor,
                            ScriptObjectKind::CelestialBody,
                            ScriptObjectKind::Actor,
                        ],
                        &[1, 1, 254],
                        &[0, 1, 2],
                        0,
                    ),
                    name => panic!("unknown navigation-candidate oracle {name}"),
                };
            let mut state = navigation_fixture(kinds, &vec![None; kinds.len()]);
            let objects = state
                .objects()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>();
            for (object, flag) in objects.iter().copied().zip(flags) {
                let field = state.object_byte(object, OBJECT_FLAGS_BYTE_OFFSET).unwrap();
                assert!(state.set_byte(field, *flag));
            }
            let source = source_indices
                .iter()
                .map(|index| objects[*index])
                .collect::<Vec<_>>();
            assert_eq!(source.len(), vector.source.len(), "{}", vector.name);
            let expected = vector
                .output
                .iter()
                .map(|offset| {
                    let source_index = vector
                        .source
                        .iter()
                        .position(|source_offset| source_offset == offset)
                        .unwrap();
                    source[source_index]
                })
                .collect::<Vec<_>>();

            assert_eq!(
                filter_navigation_candidates(&state, &source, objects[honk_index]),
                expected,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn navigation_candidates_preserve_depth_first_source_order() {
        let mut state = navigation_fixture(
            &[
                ScriptObjectKind::CelestialBody,
                ScriptObjectKind::Actor,
                ScriptObjectKind::Actor,
                ScriptObjectKind::Actor,
            ],
            &[None, Some(0), Some(1), Some(0)],
        );
        let objects = state
            .objects()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>();
        for object in &objects[1..] {
            let field = state
                .object_byte(*object, OBJECT_FLAGS_BYTE_OFFSET)
                .unwrap();
            assert!(state.set_byte(field, 1));
        }

        assert_eq!(
            navigation_candidates(&state, objects[0], objects[1]).unwrap(),
            vec![objects[2], objects[3]]
        );
    }

    #[test]
    fn arche_position_filter_matches_every_original_case() {
        let vectors: Vec<ArchePositionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_713d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ARCHE_POSITION_VECTOR_COUNT);

        for vector in vectors {
            let case = arche_position_case(&vector.name);
            let mut state = navigation_fixture(&case.kinds, &case.parents);
            let objects = state
                .objects()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>();
            for (object, flag) in objects.iter().copied().zip(&case.flags) {
                let field = state.object_byte(object, OBJECT_FLAGS_BYTE_OFFSET).unwrap();
                assert!(state.set_byte(field, *flag));
            }
            for (index, position) in case.positions {
                let field = object_word_pair(
                    &state,
                    objects[index],
                    ScriptFieldSelector::NAVIGATION_POSITION,
                )
                .unwrap();
                assert!(state.set_word_pair(field, position));
            }
            let source = case
                .source_indices
                .iter()
                .map(|index| objects[*index])
                .collect::<Vec<_>>();
            assert_eq!(source.len(), vector.processed_entries.len(), "{}", vector.name);
            let expected = vector
                .output
                .iter()
                .map(|offset| {
                    let source_index = vector
                        .processed_entries
                        .iter()
                        .position(|candidate_offset| candidate_offset == offset)
                        .unwrap();
                    source[source_index]
                })
                .collect::<Vec<_>>();

            assert_eq!(
                filter_objects_at_arche_position(&state, objects[0], &source).unwrap(),
                expected,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn arche_position_query_scans_all_decoded_objects_in_order() {
        let case = arche_position_case("kind80_linked_kind8_match");
        let mut state = navigation_fixture(&case.kinds, &case.parents);
        let objects = state
            .objects()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>();
        for (object, flag) in objects.iter().copied().zip(&case.flags) {
            let field = state.object_byte(object, OBJECT_FLAGS_BYTE_OFFSET).unwrap();
            assert!(state.set_byte(field, *flag));
        }
        for (index, position) in case.positions {
            let field = object_word_pair(
                &state,
                objects[index],
                ScriptFieldSelector::NAVIGATION_POSITION,
            )
            .unwrap();
            assert!(state.set_word_pair(field, position));
        }

        assert_eq!(
            objects_at_arche_position(&state, objects[0]).unwrap(),
            vec![objects[1], objects[2]]
        );
    }

    #[test]
    fn navigation_actor_filter_matches_every_original_case() {
        let vectors: Vec<NavigationActorTargetOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_71cf_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), NAVIGATION_ACTOR_TARGET_VECTOR_COUNT);

        for vector in vectors {
            let case = navigation_actor_target_case(&vector.name);
            let state = navigation_fixture(&case.kinds, &vec![None; case.kinds.len()]);
            let objects = state
                .objects()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>();
            let source = case
                .source_indices
                .iter()
                .map(|index| objects[*index])
                .collect::<Vec<_>>();
            assert_eq!(source.len(), vector.source.len(), "{}", vector.name);
            let expected = vector
                .output
                .iter()
                .map(|offset| {
                    let source_index = vector
                        .source
                        .iter()
                        .position(|source_offset| source_offset == offset)
                        .unwrap();
                    source[source_index]
                })
                .collect::<Vec<_>>();
            let actual = filter_navigation_actor_targets(
                &state,
                &source,
                objects[case.honk_index],
                objects[case.radio_index],
            );

            assert_eq!(actual, expected, "{}", vector.name);
            assert_eq!(actual.len(), vector.count, "{}", vector.name);
        }
    }

    #[test]
    fn navigation_actor_targets_begin_with_the_in_play_object_set() {
        let mut state = navigation_fixture(
            &[ScriptObjectKind::Actor; 4],
            &[None, None, None, None],
        );
        let objects = state
            .objects()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>();
        let flags = [2, 0, 2, 2];
        for (object, flag) in objects.iter().copied().zip(flags) {
            let field = state.object_byte(object, OBJECT_FLAGS_BYTE_OFFSET).unwrap();
            assert!(state.set_byte(field, flag));
        }

        assert_eq!(
            navigation_actor_targets(&state, objects[2], objects[3]),
            vec![objects[0]]
        );
    }

    #[test]
    fn navigation_chart_filter_matches_every_original_case() {
        let vectors: Vec<NavigationChartOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_721a_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), NAVIGATION_CHART_VECTOR_COUNT);

        for vector in vectors {
            let case = navigation_chart_case(&vector.name);
            let state = navigation_fixture(&case.kinds, &vec![None; case.kinds.len()]);
            let objects = state
                .objects()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>();
            let source_offsets = vector
                .active_object_offsets
                .iter()
                .copied()
                .take_while(|offset| (*offset as i16).is_positive() || *offset == u16::MIN)
                .collect::<Vec<_>>();
            let source = case
                .source_indices
                .iter()
                .map(|index| objects[*index])
                .collect::<Vec<_>>();
            assert_eq!(source.len(), source_offsets.len(), "{}", vector.name);
            let expected = vector
                .nav_chart_offsets
                .iter()
                .take(vector.count)
                .map(|offset| {
                    let source_index = source_offsets
                        .iter()
                        .position(|source_offset| source_offset == offset)
                        .unwrap();
                    source[source_index]
                })
                .collect::<Vec<_>>();
            let actual = filter_navigation_chart_objects(&state, &source);

            assert_eq!(actual, expected, "{}", vector.name);
            assert_eq!(actual.len(), vector.count, "{}", vector.name);
        }
    }

    #[test]
    fn navigation_chart_objects_begin_with_the_in_play_object_set() {
        let mut state = navigation_fixture(
            &[
                ScriptObjectKind::CelestialBody,
                ScriptObjectKind::NavigationEntity,
                ScriptObjectKind::BlackHole,
                ScriptObjectKind::Actor,
            ],
            &[None, None, None, None],
        );
        let objects = state
            .objects()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>();
        let flags = [2, 0, 2, 2];
        for (object, flag) in objects.iter().copied().zip(flags) {
            let field = state.object_byte(object, OBJECT_FLAGS_BYTE_OFFSET).unwrap();
            assert!(state.set_byte(field, flag));
        }

        assert_eq!(
            navigation_chart_objects(&state),
            vec![objects[0], objects[2]]
        );
    }

    #[test]
    fn every_shipped_navigation_relation_resolves_to_typed_objects() {
        for profile in 1..=5 {
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();

            for target in state.objects() {
                navigation_source_objects(&state, target.id).unwrap();
            }
            for source in state
                .objects()
                .iter()
                .filter(|object| object.kind == ScriptObjectKind::Actor)
            {
                for target in state.objects() {
                    assert!(object_links_to(&state, source.id, target.id).is_some());
                }
            }
        }
    }

    #[test]
    fn cyclic_navigation_relations_are_rejected() {
        let state = navigation_fixture(
            &[ScriptObjectKind::Actor, ScriptObjectKind::Actor],
            &[Some(1), Some(0)],
        );
        let first = state.objects()[0].id;
        assert_eq!(
            navigation_source_objects(&state, first),
            Err(ScriptNavigationError::CyclicParentRelations { object: first })
        );
    }

    #[test]
    fn position_resolution_matches_every_original_branch_vector() {
        let vectors: Vec<PositionResolverOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_61a6_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), POSITION_RESOLVER_VECTOR_COUNT);
        let mut state = navigation_fixture(
            &[
                ScriptObjectKind::CelestialBody,
                ScriptObjectKind::NavigationEntity,
                ScriptObjectKind::WorldState,
                ScriptObjectKind::Actor,
                ScriptObjectKind::CelestialBody,
                ScriptObjectKind::Actor,
                ScriptObjectKind::BlackHole,
            ],
            &[None, None, None, Some(0), None, None, None],
        );
        let objects = state
            .objects()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>();
        let arche = objects[4];
        let comparison = 30_583;
        assert!(state.set_word(
            object_field(
                &state,
                objects[6],
                ScriptFieldSelector::BLACK_HOLE_COMPARISON
            )
            .unwrap(),
            comparison
        ));

        for vector in vectors {
            let (object, expected_owner, selector, compare) = match vector.name.as_str() {
                "direct_kind8" | "direct_offset_wrap" => (
                    objects[0],
                    objects[0],
                    ScriptFieldSelector::NAVIGATION_POSITION,
                    comparison,
                ),
                "direct_kind10" => (
                    objects[1],
                    objects[1],
                    ScriptFieldSelector::NAVIGATION_POSITION,
                    comparison,
                ),
                "direct_kind200" => (
                    objects[2],
                    objects[2],
                    ScriptFieldSelector::NAVIGATION_POSITION,
                    comparison,
                ),
                "parent_link_to_direct" => (
                    objects[3],
                    objects[0],
                    ScriptFieldSelector::NAVIGATION_POSITION,
                    comparison,
                ),
                "parent_ffff_falls_back_to_arche" => (
                    objects[5],
                    arche,
                    ScriptFieldSelector::NAVIGATION_POSITION,
                    comparison,
                ),
                "kind100_match" => (
                    objects[6],
                    objects[6],
                    ScriptFieldSelector::BLACK_HOLE_MATCH_POSITION,
                    comparison,
                ),
                "kind100_mismatch" => (
                    objects[6],
                    objects[6],
                    ScriptFieldSelector::BLACK_HOLE_MISMATCH_POSITION,
                    comparison.wrapping_add(1),
                ),
                name => panic!("unknown position-resolver oracle {name}"),
            };
            assert_eq!(
                resolve_navigation_position(&state, object, arche, compare).unwrap(),
                object_word_pair(&state, expected_owner, selector).unwrap(),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn wrapped_position_math_matches_every_original_distance_vector() {
        let vectors: Vec<PositionDistanceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_60dd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), POSITION_DISTANCE_VECTOR_COUNT);

        for vector in vectors {
            let (first, second) = match vector.name.as_str() {
                "direct_kind40_three_four_five" => ([100, 100], [103, 104]),
                "delegated_direct_kind_wrap_delta_8000" => ([32_767, 5], [u16::MAX, 5]),
                "parent_ffff_falls_back_to_arche" => ([10, 10], [13, 14]),
                "first_kind100_match" => ([0, 0], [6, 8]),
                "second_kind100_mismatch" => ([4, 5], [7, 9]),
                "inherited_compare_reaches_linked_kind100" => ([1, 1], [9, 16]),
                name => panic!("unknown position-distance oracle {name}"),
            };
            assert_eq!(
                squared_distance_between_positions(first, second),
                vector.squared_distance,
                "{}",
                vector.name
            );
            assert_eq!(
                distance_between_positions(first, second),
                vector.eax as u16,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn navigation_distance_reads_live_typed_position_fields() {
        let mut state = navigation_fixture(
            &[
                ScriptObjectKind::CelestialBody,
                ScriptObjectKind::CelestialBody,
            ],
            &[None, None],
        );
        let first = state.objects()[0].id;
        let second = state.objects()[1].id;
        assert!(state.set_word_pair(
            object_word_pair(&state, first, ScriptFieldSelector::NAVIGATION_POSITION).unwrap(),
            [100, 100]
        ));
        assert!(state.set_word_pair(
            object_word_pair(&state, second, ScriptFieldSelector::NAVIGATION_POSITION).unwrap(),
            [103, 104]
        ));
        assert_eq!(
            navigation_distance(&state, first, second, first, u16::MIN).unwrap(),
            5
        );
    }

    fn arche_position_case(name: &str) -> ArchePositionCase {
        let direct_match = |kind| ArchePositionCase {
            kinds: vec![ScriptObjectKind::CelestialBody, kind],
            parents: vec![None, None],
            flags: vec![1, 1],
            positions: vec![(0, MATCHING_POSITION), (1, MATCHING_POSITION)],
            source_indices: vec![1],
        };

        match name {
            "empty_directory" => ArchePositionCase {
                kinds: vec![ScriptObjectKind::CelestialBody],
                parents: vec![None],
                flags: vec![1],
                positions: vec![(0, MATCHING_POSITION)],
                source_indices: vec![],
            },
            "direct_kind8_match" => direct_match(ScriptObjectKind::CelestialBody),
            "direct_kind10_match" => direct_match(ScriptObjectKind::NavigationEntity),
            "inactive_direct_rejected" => ArchePositionCase {
                flags: vec![1, 0],
                ..direct_match(ScriptObjectKind::CelestialBody)
            },
            "kind_mask_miss_rejected" => ArchePositionCase {
                kinds: vec![ScriptObjectKind::CelestialBody, ScriptObjectKind::Actor],
                parents: vec![None, None],
                flags: vec![1, 1],
                positions: vec![(0, MATCHING_POSITION)],
                source_indices: vec![1],
            },
            "arche_entry_excluded" => ArchePositionCase {
                kinds: vec![ScriptObjectKind::CelestialBody],
                parents: vec![None],
                flags: vec![1],
                positions: vec![(0, MATCHING_POSITION)],
                source_indices: vec![0],
            },
            "direct_position_mismatch" => ArchePositionCase {
                positions: vec![(0, MATCHING_POSITION), (1, MISMATCHING_POSITION)],
                ..direct_match(ScriptObjectKind::CelestialBody)
            },
            "kind80_linked_kind8_match" => ArchePositionCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::Location,
                    ScriptObjectKind::CelestialBody,
                ],
                parents: vec![None, Some(2), None],
                flags: vec![1, 1, 1],
                positions: vec![(0, MATCHING_POSITION), (2, MATCHING_POSITION)],
                source_indices: vec![1],
            },
            "kind80_linked_inactive_rejected" => ArchePositionCase {
                flags: vec![1, 1, 0],
                ..arche_position_case("kind80_linked_kind8_match")
            },
            "kind80_linked_kind_miss_rejected" => ArchePositionCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::Location,
                    ScriptObjectKind::Actor,
                ],
                parents: vec![None, Some(2), None],
                flags: vec![1, 1, 1],
                positions: vec![(0, MATCHING_POSITION)],
                source_indices: vec![1],
            },
            "rejected_provisional_then_accepted" => ArchePositionCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::NavigationEntity,
                ],
                parents: vec![None, None, None],
                flags: vec![1, 1, 1],
                positions: vec![
                    (0, MATCHING_POSITION),
                    (1, MISMATCHING_POSITION),
                    (2, MATCHING_POSITION),
                ],
                source_indices: vec![1, 2],
            },
            "next_low_byte_one_ignores_high_byte" => ArchePositionCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::CelestialBody,
                ],
                parents: vec![None, None, None],
                flags: vec![1, 1, 1],
                positions: vec![
                    (0, MATCHING_POSITION),
                    (1, MATCHING_POSITION),
                    (2, MATCHING_POSITION),
                ],
                source_indices: vec![1, 2],
            },
            "next_low_byte_zero_stops"
            | "record_pointer_offset_ignored_and_position_offset_wrap"
            | "directory_pointer_and_sentinel_wrap"
            | "reverse_direction_preserved" => direct_match(ScriptObjectKind::CelestialBody),
            _ => panic!("unknown Arche-position oracle {name}"),
        }
    }

    fn navigation_actor_target_case(name: &str) -> NavigationActorTargetCase {
        let actors_with_external_exclusions = |count: usize| {
            let mut kinds = vec![ScriptObjectKind::Actor; count];
            kinds.extend([ScriptObjectKind::Player, ScriptObjectKind::Player]);
            NavigationActorTargetCase {
                source_indices: (0..count).collect(),
                honk_index: count,
                radio_index: count + 1,
                kinds,
            }
        };

        match name {
            "empty_active_list" => actors_with_external_exclusions(0),
            "two_kind2_targets"
            | "zero_offset_is_valid"
            | "unsigned_high_offsets_do_not_terminate" => actors_with_external_exclusions(2),
            "exclude_honk_and_menu" => NavigationActorTargetCase {
                kinds: vec![ScriptObjectKind::Actor; 3],
                source_indices: vec![0, 1, 2],
                honk_index: 0,
                radio_index: 1,
            },
            "kind_must_equal_two" => NavigationActorTargetCase {
                kinds: vec![
                    ScriptObjectKind::Player,
                    ScriptObjectKind::BlackHole,
                    ScriptObjectKind::Actor,
                    ScriptObjectKind::Actor,
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::CelestialBody,
                ],
                source_indices: vec![0, 1, 2, 3],
                honk_index: 4,
                radio_index: 5,
            },
            "addr32_record_sum_does_not_wrap" | "inherited_reverse_direction" => {
                actors_with_external_exclusions(1)
            }
            "all_kinds_rejected" => NavigationActorTargetCase {
                kinds: vec![
                    ScriptObjectKind::Player,
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::WorldState,
                    ScriptObjectKind::NavigationEntity,
                    ScriptObjectKind::NavigationEntity,
                ],
                source_indices: vec![0, 1, 2],
                honk_index: 3,
                radio_index: 4,
            },
            _ => panic!("unknown navigation-actor-target oracle {name}"),
        }
    }

    fn navigation_chart_case(name: &str) -> NavigationChartCase {
        match name {
            "empty_list" => NavigationChartCase {
                kinds: vec![],
                source_indices: vec![],
            },
            "mixed_kind_filter" => NavigationChartCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::Actor,
                    ScriptObjectKind::BlackHole,
                    ScriptObjectKind::NavigationEntity,
                ],
                source_indices: vec![0, 1, 2, 3],
            },
            "exact_mask_and_high_bits" => NavigationChartCase {
                kinds: vec![
                    ScriptObjectKind::Player,
                    ScriptObjectKind::NavigationEntity,
                    ScriptObjectKind::BlackHole,
                    ScriptObjectKind::CelestialBody,
                ],
                source_indices: vec![0, 1, 2, 3],
            },
            "negative_8000_terminator" => NavigationChartCase {
                kinds: vec![ScriptObjectKind::CelestialBody],
                source_indices: vec![0],
            },
            "record_pointer_base_and_max_positive_offset" => NavigationChartCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::Actor,
                    ScriptObjectKind::BlackHole,
                ],
                source_indices: vec![0, 1, 2],
            },
            "helper_rebuild_precedes_scan" => NavigationChartCase {
                kinds: vec![
                    ScriptObjectKind::NavigationEntity,
                    ScriptObjectKind::Auxiliary,
                ],
                source_indices: vec![0, 1],
            },
            "inherited_reverse_direction" => NavigationChartCase {
                kinds: vec![
                    ScriptObjectKind::CelestialBody,
                    ScriptObjectKind::NavigationEntity,
                ],
                source_indices: vec![0, 1],
            },
            _ => panic!("unknown navigation-chart oracle {name}"),
        }
    }

    fn navigation_fixture(kinds: &[ScriptObjectKind], parents: &[Option<usize>]) -> ScriptState {
        assert_eq!(kinds.len(), parents.len());
        let mut offsets = Vec::with_capacity(kinds.len());
        let mut cursor = usize::MIN;
        for kind in kinds {
            offsets.push(cursor);
            cursor += kind.record_size();
        }

        let mut directory_data = Vec::new();
        let mut state_data = Vec::with_capacity(cursor);
        for (index, kind) in kinds.iter().copied().enumerate() {
            let mut directory_entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            directory_entry[0] = b'a' + u8::try_from(index).unwrap();
            directory_entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + 2]
                .copy_from_slice(&u16::try_from(offsets[index]).unwrap().to_le_bytes());
            directory_entry[DIRECTORY_NAME_CAPACITY + 2..]
                .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
            directory_data.extend_from_slice(&directory_entry);

            let mut object = vec![u8::MIN; kind.record_size()];
            object[..2].copy_from_slice(&kind.mask().to_le_bytes());
            if let Some(field_offset) =
                script_field_offset(kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
            {
                let parent = parents[index]
                    .map(|parent| u16::try_from(offsets[parent]).unwrap())
                    .unwrap_or(u16::MAX);
                object[field_offset..field_offset + 2].copy_from_slice(&parent.to_le_bytes());
            }
            state_data.extend_from_slice(&object);
        }
        directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_data).unwrap();
        decode_script_state(&state_data, &directory).unwrap()
    }
}
