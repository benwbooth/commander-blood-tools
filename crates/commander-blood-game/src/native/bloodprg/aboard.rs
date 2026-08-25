//! Objects whose relationship field currently denotes the ship interior.

use commander_blood_formats::script::ScriptObjectId;

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::script::{decode_script_directory, ScriptDirectory};
    use serde::Deserialize;

    use super::*;

    const REMOVE_VECTOR_COUNT: usize = 6;
    const INSERT_VECTOR_COUNT: usize = 7;

    #[derive(Deserialize)]
    struct RosterOracle {
        owner: u16,
        slots_before: Vec<u16>,
        slots_after: Vec<u16>,
        success_carry: bool,
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
}
