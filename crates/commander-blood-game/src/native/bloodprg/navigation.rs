//! Typed object relationships used by BloodScript ship-navigation handlers.

use std::collections::BTreeSet;
use std::fmt;

use commander_blood_formats::script::{ScriptObjectId, ScriptState, ScriptStateObjectReference};

use super::{ScriptFieldSelector, script_field_offset};

const BITS_PER_BYTE: usize = u8::BITS as usize;

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
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;

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
