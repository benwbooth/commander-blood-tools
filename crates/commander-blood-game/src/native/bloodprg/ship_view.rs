//! Typed ship-view artwork selection and staging.

use std::fmt;

use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState,
};
use commander_blood_formats::world_art::WorldArtworkLayout;

use super::{ScriptFieldSelector, script_field_offset};

const SHIP_VIEW_TRANSITION_ENTITY: u16 = 31;
const OFFSCREEN_COORDINATE: i16 = -1_000;
const INITIAL_ENTITY_FRAME: u16 = 0;

/// Typed render-entity identity used by the ship-view host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipViewEntityId(u16);

impl ShipViewEntityId {
    /// Construct an entity identity recovered from native data.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the native entity-table index.
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// Typed resource-table identity used by the ship-view host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipViewResourceId(u16);

impl ShipViewResourceId {
    /// Construct a resource identity recovered from the embedded layout.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the original resource-table index.
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// Resource load requested for a selected world-artwork row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipViewResourceRequest {
    /// Resource selected by object display name.
    pub resource: ShipViewResourceId,
    /// Native high-priority load bit, represented without packing it into the ID.
    pub prioritized: bool,
}

/// Entity placement staged after the artwork load attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipViewEntityPlacement {
    /// Entity owned by the selected layout row.
    pub entity: ShipViewEntityId,
    /// Initial offscreen position used before the transition animates it.
    pub position: [i16; 2],
    /// Initial animation frame.
    pub frame: u16,
}

/// Complete observable result of one ship-view artwork selection pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipViewArtworkSelection {
    /// Entity transitioned before any object scan.
    pub transitioned_entity: ShipViewEntityId,
    /// Position-matching objects in authored directory order.
    pub matching_objects: Vec<ScriptObjectId>,
    /// Activated world-artwork layout row.
    pub selected_layout: Option<usize>,
    /// Resource request emitted for the selected row.
    pub resource_request: Option<ShipViewResourceRequest>,
    /// Entity placement emitted even if the host resource load fails.
    pub entity_placement: Option<ShipViewEntityPlacement>,
}

/// Invalid typed state encountered while selecting ship-view artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipViewArtworkError {
    /// The current navigation object is not present in the decoded profile.
    MissingCurrentObject {
        /// Missing object identity.
        object: ScriptObjectId,
    },
    /// The current navigation object has no proven position field.
    MissingCurrentPosition {
        /// Object lacking a position.
        object: ScriptObjectId,
    },
    /// A black-hole object lacks one of its two proven comparison positions.
    MissingBlackHolePosition {
        /// Malformed black-hole object.
        object: ScriptObjectId,
    },
    /// A selected object has no corresponding active directory entry.
    MissingDirectoryObject {
        /// Object lacking a name entry.
        object: ScriptObjectId,
    },
}

impl fmt::Display for ShipViewArtworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ShipViewArtworkError {}

/// Select and stage world artwork for objects sharing the current position.
///
/// This is the flat-memory translation of `draw_hud_element_2bc7` at executable
/// offset `0x006FF3`. The native scratch-offset list becomes owned object IDs,
/// and resource loading is returned as a host request so a failed load cannot
/// suppress the original unconditional entity placement.
pub fn select_ship_view_artwork(
    directory: &ScriptDirectory,
    state: &ScriptState,
    current: ScriptObjectId,
    layouts: &mut [WorldArtworkLayout],
) -> Result<ShipViewArtworkSelection, ShipViewArtworkError> {
    for layout in layouts.iter_mut() {
        layout.active = false;
    }

    let current_kind = state
        .object(current)
        .ok_or(ShipViewArtworkError::MissingCurrentObject { object: current })?
        .kind;
    let current_position = object_position(
        state,
        current,
        current_kind,
        ScriptFieldSelector::NAVIGATION_POSITION,
    )
    .ok_or(ShipViewArtworkError::MissingCurrentPosition { object: current })?;

    let mut matching_objects = Vec::new();
    for (object, _entry) in directory.active_objects() {
        if object == current {
            continue;
        }
        let Some(state_object) = state.object(object) else {
            continue;
        };
        let matched = if state_object.kind == ScriptObjectKind::BlackHole {
            let first = object_position(
                state,
                object,
                state_object.kind,
                ScriptFieldSelector::BLACK_HOLE_MATCH_POSITION,
            )
            .ok_or(ShipViewArtworkError::MissingBlackHolePosition { object })?;
            let second = object_position(
                state,
                object,
                state_object.kind,
                ScriptFieldSelector::BLACK_HOLE_MISMATCH_POSITION,
            )
            .ok_or(ShipViewArtworkError::MissingBlackHolePosition { object })?;
            first == current_position || second == current_position
        } else {
            object_position(
                state,
                object,
                state_object.kind,
                ScriptFieldSelector::NAVIGATION_POSITION,
            ) == Some(current_position)
        };
        if matched {
            matching_objects.push(object);
        }
    }

    let selected_layout = matching_objects
        .first()
        .copied()
        .map(|object| {
            let name = directory
                .object(object)
                .ok_or(ShipViewArtworkError::MissingDirectoryObject { object })?
                .name();
            Ok(layouts.iter().position(|layout| layout.name() == name))
        })
        .transpose()?
        .flatten();

    let (resource_request, entity_placement) = if let Some(index) = selected_layout {
        let layout = &mut layouts[index];
        layout.active = true;
        (
            Some(ShipViewResourceRequest {
                resource: ShipViewResourceId::new(layout.resource_id),
                prioritized: true,
            }),
            Some(ShipViewEntityPlacement {
                entity: ShipViewEntityId::new(layout.entity_id),
                position: [OFFSCREEN_COORDINATE; 2],
                frame: INITIAL_ENTITY_FRAME,
            }),
        )
    } else {
        (None, None)
    };

    Ok(ShipViewArtworkSelection {
        transitioned_entity: ShipViewEntityId::new(SHIP_VIEW_TRANSITION_ENTITY),
        matching_objects,
        selected_layout,
        resource_request,
        entity_placement,
    })
}

fn object_position(
    state: &ScriptState,
    object: ScriptObjectId,
    kind: ScriptObjectKind,
    selector: ScriptFieldSelector,
) -> Option<[u16; 2]> {
    let byte_offset = script_field_offset(kind, selector)?;
    let field = state.object_word_pair(object, byte_offset / std::mem::size_of::<u16>())?;
    state.word_pair(field)
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::script::{decode_script_directory, decode_script_state};
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 16;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const RESOURCE_ID_MASK: u16 = 0x7FFF;
    const CURRENT_POSITION: [u16; 2] = [0x2468, 0x1357];
    const OTHER_POSITION: [u16; 2] = [0x1357, 0x2468];

    #[derive(Deserialize)]
    struct ArtworkOracle {
        name: String,
        matches: Vec<u16>,
        selected_layout: Option<usize>,
        helper_calls: Vec<HelperOracle>,
    }

    #[derive(Deserialize)]
    struct HelperOracle {
        call: String,
        resource: Option<serde_json::Value>,
        entity: Option<u16>,
    }

    struct ObjectFixture {
        kind: ScriptObjectKind,
        name: &'static [u8],
        positions: Vec<[u16; 2]>,
    }

    #[test]
    fn ship_view_artwork_selection_matches_every_original_branch_vector() {
        let vectors: Vec<ArtworkOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6ff3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let (objects, layout_names) = scenario(&vector.name);
            let (directory, state) = state_fixture(&objects);
            let object_ids = state
                .objects()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>();
            let mut layouts = layout_names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let selected = vector.selected_layout == Some(index);
                    let resource = selected
                        .then(|| helper_value(&vector.helper_calls, "resource_named_file_load"))
                        .flatten()
                        .map(|value| value & RESOURCE_ID_MASK)
                        .unwrap_or(u16::try_from(index + 1).unwrap());
                    let entity = selected
                        .then(|| helper_entity(&vector.helper_calls))
                        .flatten()
                        .unwrap_or(u16::try_from(index + 1).unwrap());
                    WorldArtworkLayout::new(*name, resource, entity, true).unwrap()
                })
                .collect::<Vec<_>>();

            let outcome =
                select_ship_view_artwork(&directory, &state, object_ids[0], &mut layouts).unwrap();

            assert_eq!(
                outcome.transitioned_entity.value(),
                SHIP_VIEW_TRANSITION_ENTITY,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.matching_objects.len(),
                vector.matches.len(),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.matching_objects,
                object_ids
                    .iter()
                    .copied()
                    .skip(1)
                    .take(vector.matches.len())
                    .collect::<Vec<_>>(),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.selected_layout, vector.selected_layout,
                "{}",
                vector.name
            );
            for (index, layout) in layouts.iter().enumerate() {
                assert_eq!(
                    layout.active,
                    vector.selected_layout == Some(index),
                    "{} layout {index}",
                    vector.name
                );
            }

            if let Some(selected) = vector.selected_layout {
                let request = outcome.resource_request.unwrap();
                assert_eq!(request.resource.value(), layouts[selected].resource_id);
                assert!(request.prioritized);
                let placement = outcome.entity_placement.unwrap();
                assert_eq!(placement.entity.value(), layouts[selected].entity_id);
                assert_eq!(placement.position, [OFFSCREEN_COORDINATE; 2]);
                assert_eq!(placement.frame, INITIAL_ENTITY_FRAME);
            } else {
                assert_eq!(outcome.resource_request, None);
                assert_eq!(outcome.entity_placement, None);
            }
        }
    }

    fn scenario(name: &str) -> (Vec<ObjectFixture>, Vec<&'static [u8]>) {
        let current = ObjectFixture {
            kind: ScriptObjectKind::NavigationEntity,
            name: b"CURRENT",
            positions: vec![CURRENT_POSITION],
        };
        let (candidates, layouts) = match name {
            "only_current_object_has_no_match" => (vec![], vec![b"PLANET".as_slice()]),
            "direct_position_match_loads_named_layout" => (
                vec![direct(b"PLANET", CURRENT_POSITION)],
                vec![b"PLANET".as_slice(), b"OTHER".as_slice()],
            ),
            "direct_position_mismatch" => (
                vec![direct(b"PLANET", OTHER_POSITION)],
                vec![b"PLANET".as_slice()],
            ),
            "direct_zero_field_offset_is_rejected" => (
                vec![ObjectFixture {
                    kind: ScriptObjectKind::Actor,
                    name: b"PLANET",
                    positions: vec![],
                }],
                vec![b"PLANET".as_slice()],
            ),
            "kind100_first_position_matches" => (
                vec![black_hole(CURRENT_POSITION, OTHER_POSITION)],
                vec![b"STATION".as_slice()],
            ),
            "kind100_second_position_matches" => (
                vec![black_hole(OTHER_POSITION, CURRENT_POSITION)],
                vec![b"STATION".as_slice()],
            ),
            "kind100_both_positions_mismatch" => (
                vec![black_hole(OTHER_POSITION, [u16::MAX; 2])],
                vec![b"STATION".as_slice()],
            ),
            "multiple_matches_select_first_directory_object" => (
                vec![
                    direct(b"SECOND", CURRENT_POSITION),
                    direct(b"FIRST", CURRENT_POSITION),
                ],
                vec![b"FIRST".as_slice(), b"SECOND".as_slice()],
            ),
            "matching_position_without_layout_name" => (
                vec![direct(b"UNKNOWN", CURRENT_POSITION)],
                vec![b"PLANET".as_slice()],
            ),
            "empty_layout_table_after_position_match" => {
                (vec![direct(b"PLANET", CURRENT_POSITION)], vec![])
            }
            "resource_failure_result_is_ignored" => (
                vec![direct(b"PLANET", CURRENT_POSITION)],
                vec![b"PLANET".as_slice()],
            ),
            "nonactive_next_directory_entry_stops_scan" => (
                vec![direct(b"FIRST", CURRENT_POSITION)],
                vec![b"FIRST".as_slice(), b"SECOND".as_slice()],
            ),
            "negative_signed_position_offsets" => (
                vec![direct(b"NEGATIVE", CURRENT_POSITION)],
                vec![b"NEGATIVE".as_slice()],
            ),
            "addr32_position_crosses_64k_without_wrap" => (
                vec![direct(b"CROSS", CURRENT_POSITION)],
                vec![b"CROSS".as_slice()],
            ),
            "addr32_position_inherits_upper_esi" => (
                vec![direct(b"UPPER", CURRENT_POSITION)],
                vec![b"UPPER".as_slice()],
            ),
            "initial_ds_clear_is_distinct_from_later_gs_layout" => (
                vec![direct(b"PLANET", CURRENT_POSITION)],
                vec![b"PLANET".as_slice(), b"OTHER".as_slice()],
            ),
            unknown => panic!("unknown 0x006FF3 oracle case {unknown}"),
        };
        let mut objects = vec![current];
        objects.extend(candidates);
        (objects, layouts)
    }

    fn direct(name: &'static [u8], position: [u16; 2]) -> ObjectFixture {
        ObjectFixture {
            kind: ScriptObjectKind::CelestialBody,
            name,
            positions: vec![position],
        }
    }

    fn black_hole(first: [u16; 2], second: [u16; 2]) -> ObjectFixture {
        ObjectFixture {
            kind: ScriptObjectKind::BlackHole,
            name: b"STATION",
            positions: vec![first, second],
        }
    }

    fn helper_value(calls: &[HelperOracle], name: &str) -> Option<u16> {
        calls
            .iter()
            .find(|call| call.call == name)
            .and_then(|call| call.resource.as_ref())
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
    }

    fn helper_entity(calls: &[HelperOracle]) -> Option<u16> {
        calls
            .iter()
            .find(|call| call.call == "entity_record_setter")
            .and_then(|call| call.entity)
    }

    fn state_fixture(objects: &[ObjectFixture]) -> (ScriptDirectory, ScriptState) {
        let mut directory_data = Vec::new();
        let mut state_data = Vec::new();

        for object in objects {
            let mut entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            entry[..object.name.len()].copy_from_slice(object.name);
            entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + 2]
                .copy_from_slice(&u16::try_from(state_data.len()).unwrap().to_le_bytes());
            entry[DIRECTORY_NAME_CAPACITY + 2..]
                .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
            directory_data.extend_from_slice(&entry);

            let mut record = vec![u8::MIN; object.kind.record_size()];
            record[..2].copy_from_slice(&object.kind.mask().to_le_bytes());
            if object.kind == ScriptObjectKind::BlackHole {
                write_position(
                    &mut record,
                    object.kind,
                    ScriptFieldSelector::BLACK_HOLE_MATCH_POSITION,
                    object.positions[0],
                );
                write_position(
                    &mut record,
                    object.kind,
                    ScriptFieldSelector::BLACK_HOLE_MISMATCH_POSITION,
                    object.positions[1],
                );
            } else if let Some(position) = object.positions.first().copied() {
                write_position(
                    &mut record,
                    object.kind,
                    ScriptFieldSelector::NAVIGATION_POSITION,
                    position,
                );
            }
            state_data.extend_from_slice(&record);
        }
        directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_data).unwrap();
        let state = decode_script_state(&state_data, &directory).unwrap();
        (directory, state)
    }

    fn write_position(
        record: &mut [u8],
        kind: ScriptObjectKind,
        selector: ScriptFieldSelector,
        position: [u16; 2],
    ) {
        let offset = script_field_offset(kind, selector).unwrap();
        record[offset..offset + 2].copy_from_slice(&position[0].to_le_bytes());
        record[offset + 2..offset + 4].copy_from_slice(&position[1].to_le_bytes());
    }
}
