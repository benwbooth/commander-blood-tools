//! Objects whose relationship field currently denotes the ship interior.

use std::fmt;

use commander_blood_formats::script::{ScriptObjectId, ScriptState, ScriptStateObjectReference};

use super::{ScriptFieldSelector, script_field_offset};

/// Maximum number of object owners tracked by the original game state.
pub const ABOARD_OBJECT_CAPACITY: usize = 16;

const INITIAL_OBJECT_INDEX: usize = 0;

/// Fixed-capacity roster of objects currently represented by the aboard sentinel.
///
/// `Option` makes empty entries distinct from object identity. The original
/// serialized representation used zero for both an empty entry and the first
/// object's encoded identity; the insertion and removal functions retain that
/// observable first-object behavior without retaining a sentinel-valued array.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AboardObjectRoster {
    slots: [Option<ScriptObjectId>; ABOARD_OBJECT_CAPACITY],
}

/// Invalid typed state encountered while rebuilding the aboard roster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AboardRosterError {
    /// More sentinel-owned objects exist than the original roster can hold.
    CapacityExceeded {
        /// Number of objects requiring roster entries.
        required: usize,
    },
}

impl fmt::Display for AboardRosterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AboardRosterError {}

impl AboardObjectRoster {
    /// Return roster entries in their stable authored priority order.
    pub fn slots(&self) -> &[Option<ScriptObjectId>; ABOARD_OBJECT_CAPACITY] {
        &self.slots
    }

    fn matching_slot(&self, owner: ScriptObjectId) -> Option<usize> {
        if owner.index() == INITIAL_OBJECT_INDEX {
            self.slots.iter().position(Option::is_none)
        } else {
            self.slots.iter().position(|entry| *entry == Some(owner))
        }
    }
}

/// Remove the first matching owner from the aboard-object roster.
///
/// This translates `vm_special_slot_remove` at BLOODPRG file offset `0x005FD8`.
pub fn remove_aboard_object(roster: &mut AboardObjectRoster, owner: ScriptObjectId) -> bool {
    let Some(index) = roster.matching_slot(owner) else {
        return false;
    };
    roster.slots[index] = None;
    true
}

/// Keep an existing owner or insert it into the first available roster entry.
///
/// This translates `vm_special_slot_insert` at BLOODPRG file offset `0x005FF6`.
/// Duplicate detection intentionally precedes the empty-entry search.
pub fn insert_aboard_object(roster: &mut AboardObjectRoster, owner: ScriptObjectId) -> bool {
    if roster.matching_slot(owner).is_some() {
        return true;
    }
    let Some(index) = roster.slots.iter().position(Option::is_none) else {
        return false;
    };
    roster.slots[index] = Some(owner);
    true
}

/// Rebuild the aboard roster from typed object-holder relations.
///
/// This translates `vm_record_state_proc` at BLOODPRG file offset `0x00555B`.
/// The native routine reused zero and `0xFFFF` as competing slot sentinels;
/// this flat model derives the complete roster from decoded object identities
/// and commits it only when every aboard object fits.
pub fn rebuild_aboard_roster(
    state: &ScriptState,
    roster: &mut AboardObjectRoster,
) -> Result<usize, AboardRosterError> {
    let mut aboard = Vec::new();
    for object in state.objects() {
        let Some(field_offset) =
            script_field_offset(object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
        else {
            continue;
        };
        let Some(field) = state.object_word(object.id, field_offset / std::mem::size_of::<u16>())
        else {
            continue;
        };
        if state.object_reference(field) == Some(ScriptStateObjectReference::Sentinel) {
            aboard.push(object.id);
        }
    }

    if aboard.len() > ABOARD_OBJECT_CAPACITY {
        return Err(AboardRosterError::CapacityExceeded {
            required: aboard.len(),
        });
    }
    roster.slots.fill(None);
    for (slot, object) in roster.slots.iter_mut().zip(aboard.iter().copied()) {
        *slot = Some(object);
    }
    Ok(aboard.len())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::script::{
        ScriptDirectory, ScriptObjectKind, ScriptState, decode_script_directory,
        decode_script_state,
    };
    use serde::Deserialize;

    use super::*;

    const REMOVE_VECTOR_COUNT: usize = 6;
    const INSERT_VECTOR_COUNT: usize = 7;
    const REBUILD_VECTOR_COUNT: usize = 10;
    const ORIGINAL_PROFILE_COUNT: usize = 5;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();
    const NATIVE_ABOARD_SENTINEL: i16 = -1;

    #[derive(Deserialize)]
    struct RosterOracle {
        owner: u16,
        slots_before: Vec<u16>,
        slots_after: Vec<u16>,
        success_carry: bool,
    }

    #[derive(Deserialize)]
    struct RebuildOracle {
        name: String,
        entries: Vec<RebuildOracleEntry>,
        scanned_entries: usize,
        slots_after: Vec<u16>,
    }

    #[derive(Deserialize)]
    struct RebuildOracleEntry {
        object_offset: u16,
        field_value: i16,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn directory() -> ScriptDirectory {
        decode_script_directory(&std::fs::read(original_asset("SCRIPT1.DEB")).unwrap()).unwrap()
    }

    fn translated_roster(
        vector: &RosterOracle,
        directory: &ScriptDirectory,
    ) -> (
        ScriptObjectId,
        [Option<ScriptObjectId>; ABOARD_OBJECT_CAPACITY],
        [Option<ScriptObjectId>; ABOARD_OBJECT_CAPACITY],
    ) {
        let object_ids = directory
            .active_objects()
            .map(|(object, _entry)| object)
            .collect::<Vec<_>>();
        let mut identities = BTreeMap::from([(u16::MIN, object_ids[INITIAL_OBJECT_INDEX])]);
        let mut next_identity = INITIAL_OBJECT_INDEX + 1;
        for value in std::iter::once(vector.owner)
            .chain(vector.slots_before.iter().copied())
            .chain(vector.slots_after.iter().copied())
            .filter(|value| *value != u16::MIN)
        {
            identities.entry(value).or_insert_with(|| {
                let object = object_ids[next_identity];
                next_identity += 1;
                object
            });
        }
        let decode_slots = |values: &[u16]| {
            values
                .iter()
                .map(|value| {
                    (*value != u16::MIN).then(|| *identities.get(value).expect("mapped identity"))
                })
                .collect::<Vec<_>>()
                .try_into()
                .expect("fixed roster size")
        };
        (
            identities[&vector.owner],
            decode_slots(&vector.slots_before),
            decode_slots(&vector.slots_after),
        )
    }

    fn actor_state(holder_is_aboard: &[bool]) -> ScriptState {
        let kind = ScriptObjectKind::Actor;
        let field_offset = script_field_offset(kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
            .expect("actors have a holder field");
        let mut directory_data = Vec::new();
        let mut state_data = Vec::new();

        for (index, is_aboard) in holder_is_aboard.iter().copied().enumerate() {
            let source_offset = index * kind.record_size();
            let mut directory_entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            directory_entry[usize::MIN] = b'a' + u8::try_from(index).unwrap();
            directory_entry
                [DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE]
                .copy_from_slice(&u16::try_from(source_offset).unwrap().to_le_bytes());
            directory_entry[DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE..]
                .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
            directory_data.extend_from_slice(&directory_entry);

            let mut object = vec![u8::MIN; kind.record_size()];
            object[..SERIALIZED_WORD_SIZE].copy_from_slice(&kind.mask().to_le_bytes());
            let encoded_holder = if is_aboard {
                u16::MAX
            } else {
                u16::try_from(source_offset).unwrap()
            };
            object[field_offset..field_offset + SERIALIZED_WORD_SIZE]
                .copy_from_slice(&encoded_holder.to_le_bytes());
            state_data.extend_from_slice(&object);
        }
        directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_data).unwrap();
        decode_script_state(&state_data, &directory).unwrap()
    }

    #[test]
    fn removal_matches_every_original_roster_vector() {
        let vectors: Vec<RosterOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5fd8_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), REMOVE_VECTOR_COUNT);
        let directory = directory();

        for vector in vectors {
            let (owner, slots_before, slots_after) = translated_roster(&vector, &directory);
            let mut roster = AboardObjectRoster {
                slots: slots_before,
            };

            assert_eq!(
                remove_aboard_object(&mut roster, owner),
                vector.success_carry
            );
            assert_eq!(roster.slots(), &slots_after);
        }
    }

    #[test]
    fn insertion_matches_every_original_roster_vector() {
        let vectors: Vec<RosterOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5ff6_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), INSERT_VECTOR_COUNT);
        let directory = directory();

        for vector in vectors {
            let (owner, slots_before, slots_after) = translated_roster(&vector, &directory);
            let mut roster = AboardObjectRoster {
                slots: slots_before,
            };

            assert_eq!(
                insert_aboard_object(&mut roster, owner),
                vector.success_carry
            );
            assert_eq!(roster.slots(), &slots_after);
        }
    }

    #[test]
    fn rebuild_accounts_for_every_natural_vector_in_the_flat_semantic_domain() {
        let vectors: Vec<RebuildOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_555b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), REBUILD_VECTOR_COUNT);

        for vector in vectors {
            let scanned = &vector.entries[..vector.scanned_entries];
            let aboard_flags = scanned
                .iter()
                .map(|entry| entry.field_value == NATIVE_ABOARD_SENTINEL)
                .collect::<Vec<_>>();
            let expected_offsets = scanned
                .iter()
                .filter(|entry| entry.field_value == NATIVE_ABOARD_SENTINEL)
                .map(|entry| entry.object_offset)
                .collect::<Vec<_>>();
            assert_eq!(
                &vector.slots_after[..expected_offsets.len()],
                expected_offsets,
                "{}",
                vector.name
            );
            let state = actor_state(&aboard_flags);
            let expected_objects = state
                .objects()
                .iter()
                .zip(aboard_flags.iter().copied())
                .filter_map(|(object, is_aboard)| is_aboard.then_some(object.id))
                .collect::<Vec<_>>();
            let mut roster = AboardObjectRoster::default();

            assert_eq!(
                rebuild_aboard_roster(&state, &mut roster).unwrap(),
                expected_objects.len(),
                "{}",
                vector.name
            );
            assert_eq!(
                roster.slots().iter().flatten().copied().collect::<Vec<_>>(),
                expected_objects,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn every_shipped_profile_rebuilds_the_complete_typed_roster() {
        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();
            let expected = state
                .objects()
                .iter()
                .filter_map(|object| {
                    let offset =
                        script_field_offset(object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)?;
                    let field =
                        state.object_word(object.id, offset / std::mem::size_of::<u16>())?;
                    (state.object_reference(field) == Some(ScriptStateObjectReference::Sentinel))
                        .then_some(object.id)
                })
                .collect::<Vec<_>>();
            let mut roster = AboardObjectRoster::default();

            assert_eq!(
                rebuild_aboard_roster(&state, &mut roster).unwrap(),
                expected.len()
            );
            assert_eq!(
                roster.slots().iter().flatten().copied().collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn oversized_rebuild_is_rejected_without_changing_the_roster() {
        const EXCESS_OBJECT_COUNT: usize = ABOARD_OBJECT_CAPACITY + 1;

        let state = actor_state(&[true; EXCESS_OBJECT_COUNT]);
        let existing = state.objects()[usize::MIN].id;
        let mut roster = AboardObjectRoster::default();
        assert!(insert_aboard_object(&mut roster, existing));
        let before = roster.clone();

        assert_eq!(
            rebuild_aboard_roster(&state, &mut roster).unwrap_err(),
            AboardRosterError::CapacityExceeded {
                required: EXCESS_OBJECT_COUNT,
            }
        );
        assert_eq!(roster, before);
    }
}
