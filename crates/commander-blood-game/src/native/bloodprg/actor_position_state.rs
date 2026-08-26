//! Actor flag normalization derived from live navigation positions.

use std::fmt;

use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptState};

use super::{PresentationRequestFlags, ScriptNavigationError, resolve_navigation_position};

const OBJECT_FLAGS_WORD_INDEX: usize = 1;
const POSITION_MATCH_FLAG: u16 = 0x0010;
const POST_UPDATE_FLAG: u16 = 0x8000;
const TRANSIENT_ACTOR_FLAGS: u16 = POSITION_MATCH_FLAG | POST_UPDATE_FLAG;

/// Runtime inputs controlling one actor-position normalization pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorPositionStateContext {
    /// Pending text or sequence work that preserves transient actor bits.
    pub request_flags: PresentationRequestFlags,
    /// Whether authored text presentation is currently active.
    pub text_display_active: bool,
    /// Built-in Honk object used by the active-text exception.
    pub honk: Option<ScriptObjectId>,
    /// Object currently marked for post-update processing.
    pub post_update: Option<ScriptObjectId>,
    /// Built-in `orxx` world-state object.
    pub world: ScriptObjectId,
    /// Built-in `arche` position fallback.
    pub arche: ScriptObjectId,
}

/// Summary of one completed actor-position normalization pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorPositionStateOutcome {
    /// Number of actor records visited.
    pub processed_actors: usize,
    /// Number whose resolved position matched `orxx` or `arche`.
    pub matching_positions: usize,
}

/// Invalid typed state encountered before actor updates could be committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorPositionStateError {
    /// An actor's fixed header does not contain its flags word.
    MissingFlags {
        /// Actor with the malformed record.
        object: ScriptObjectId,
    },
    /// A resolved coordinate pair could not be read from owned state.
    MissingPosition {
        /// Object whose resolved coordinate pair was unreadable.
        object: ScriptObjectId,
    },
    /// Typed holder or coordinate traversal failed.
    Navigation(ScriptNavigationError),
}

impl fmt::Display for ActorPositionStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ActorPositionStateError {}

impl From<ScriptNavigationError> for ActorPositionStateError {
    fn from(source: ScriptNavigationError) -> Self {
        Self::Navigation(source)
    }
}

/// Normalize every actor's transient flags against current navigation positions.
///
/// This translates `vm_state_processor` at BLOODPRG file offset `0x005A74`.
/// Stable object identities and typed coordinate pairs replace the original
/// directory walk and record-address rebinding.
pub fn update_actor_position_states(
    state: &mut ScriptState,
    context: ActorPositionStateContext,
) -> Result<ActorPositionStateOutcome, ActorPositionStateError> {
    let actor_ids = state
        .objects()
        .iter()
        .filter(|object| object.kind == ScriptObjectKind::Actor)
        .map(|object| object.id)
        .collect::<Vec<_>>();
    let mut updates = Vec::with_capacity(actor_ids.len());
    let mut outcome = ActorPositionStateOutcome::default();

    for actor in actor_ids {
        let flags_field = state
            .object_word(actor, OBJECT_FLAGS_WORD_INDEX)
            .ok_or(ActorPositionStateError::MissingFlags { object: actor })?;
        let mut flags = state
            .word(flags_field)
            .ok_or(ActorPositionStateError::MissingFlags { object: actor })?;
        if !context.request_flags.any_request_pending()
            && (!context.text_display_active
                || (context.honk == Some(actor) && context.post_update == Some(actor)))
        {
            flags &= !TRANSIENT_ACTOR_FLAGS;
        }

        let actor_position = resolved_position(state, actor, context.arche, flags)?;
        let world_position = resolved_position(state, context.world, context.arche, flags)?;
        let matches_position = if actor_position == world_position {
            true
        } else {
            actor_position == resolved_position(state, context.arche, context.arche, flags)?
        };
        if matches_position {
            flags |= POSITION_MATCH_FLAG;
            outcome.matching_positions += 1;
        }
        outcome.processed_actors += 1;
        updates.push((flags_field, flags));
    }

    for (field, flags) in updates {
        let assigned = state.set_word(field, flags);
        debug_assert!(assigned, "validated actor flags remain writable");
    }
    Ok(outcome)
}

fn resolved_position(
    state: &ScriptState,
    object: ScriptObjectId,
    arche: ScriptObjectId,
    black_hole_compare: u16,
) -> Result<[u16; 2], ActorPositionStateError> {
    let field = resolve_navigation_position(state, object, arche, black_hole_compare)?;
    state
        .word_pair(field)
        .ok_or(ActorPositionStateError::MissingPosition { object })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::script::{decode_script_directory, decode_script_state};
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{ScriptFieldSelector, script_field_offset};

    const ORACLE_VECTOR_COUNT: usize = 14;
    const ORIGINAL_PROFILE_COUNT: usize = 5;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();
    const POSITION_A: [u16; 2] = [100, 200];
    const POSITION_B: [u16; 2] = [300, 400];
    const POSITION_C: [u16; 2] = [500, 600];
    const BUILTIN_WORLD_NAME: &[u8] = b"orxx";
    const BUILTIN_ARCHETYPE_NAME: &[u8] = b"arche";
    const BUILTIN_HONK_NAME: &[u8] = b"Honk";

    #[derive(Deserialize)]
    struct StateProcessorOracle {
        name: String,
        request_flags: u8,
        text_active: u8,
        processed_entries: Vec<u16>,
        state_words_after: BTreeMap<String, u16>,
        resolver_calls: Vec<[u16; 3]>,
    }

    struct Fixture {
        state: ScriptState,
        actor: Option<ScriptObjectId>,
        world: ScriptObjectId,
        arche: ScriptObjectId,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn fixture(
        actor_flags: Option<u16>,
        actor_position: [u16; 2],
        world_position: [u16; 2],
        arche_position: [u16; 2],
        leading_non_actor: bool,
    ) -> Fixture {
        let mut kinds = Vec::new();
        if leading_non_actor {
            kinds.push(ScriptObjectKind::InventoryItem);
        }
        let actor_index = actor_flags.map(|_| {
            let index = kinds.len();
            kinds.push(ScriptObjectKind::Actor);
            index
        });
        let world_index = kinds.len();
        kinds.push(ScriptObjectKind::WorldState);
        let arche_index = kinds.len();
        kinds.push(ScriptObjectKind::NavigationEntity);
        let anchor_index = kinds.len();
        kinds.push(ScriptObjectKind::NavigationEntity);

        let mut offsets = Vec::with_capacity(kinds.len());
        let mut cursor = usize::MIN;
        for kind in &kinds {
            offsets.push(cursor);
            cursor += kind.record_size();
        }
        let mut directory_data = Vec::new();
        let mut state_data = Vec::with_capacity(cursor);
        for (index, kind) in kinds.iter().copied().enumerate() {
            let mut entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            entry[usize::MIN] = b'a' + u8::try_from(index).unwrap();
            entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE]
                .copy_from_slice(&u16::try_from(offsets[index]).unwrap().to_le_bytes());
            entry[DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE..]
                .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
            directory_data.extend_from_slice(&entry);

            let mut object = vec![u8::MIN; kind.record_size()];
            object[..SERIALIZED_WORD_SIZE].copy_from_slice(&kind.mask().to_le_bytes());
            if Some(index) == actor_index {
                object[SERIALIZED_WORD_SIZE..SERIALIZED_WORD_SIZE * 2]
                    .copy_from_slice(&actor_flags.unwrap().to_le_bytes());
                let holder =
                    script_field_offset(kind, ScriptFieldSelector::HOLDER_OR_LOCATION).unwrap();
                object[holder..holder + SERIALIZED_WORD_SIZE]
                    .copy_from_slice(&u16::try_from(offsets[anchor_index]).unwrap().to_le_bytes());
            }
            if matches!(
                kind,
                ScriptObjectKind::WorldState | ScriptObjectKind::NavigationEntity
            ) {
                let position =
                    script_field_offset(kind, ScriptFieldSelector::NAVIGATION_POSITION).unwrap();
                let value = if index == world_index {
                    world_position
                } else if index == arche_index {
                    arche_position
                } else {
                    actor_position
                };
                object[position..position + SERIALIZED_WORD_SIZE]
                    .copy_from_slice(&value[0].to_le_bytes());
                object[position + SERIALIZED_WORD_SIZE..position + SERIALIZED_WORD_SIZE * 2]
                    .copy_from_slice(&value[1].to_le_bytes());
            }
            state_data.extend_from_slice(&object);
        }
        directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_data).unwrap();
        let state = decode_script_state(&state_data, &directory).unwrap();
        Fixture {
            actor: actor_index.map(|index| state.objects()[index].id),
            world: state.objects()[world_index].id,
            arche: state.objects()[arche_index].id,
            state,
        }
    }

    fn actor_flags(state: &ScriptState, actor: ScriptObjectId) -> u16 {
        state
            .word(state.object_word(actor, OBJECT_FLAGS_WORD_INDEX).unwrap())
            .unwrap()
    }

    #[test]
    fn state_processor_accounts_for_every_original_natural_vector() {
        let vectors: Vec<StateProcessorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5a74_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let expected_flags = vector
                .state_words_after
                .values()
                .next_back()
                .copied()
                .unwrap();
            let actor_base_flags = vector.resolver_calls.first().map(|call| call[1]);
            let request_flags = PresentationRequestFlags::decode(vector.request_flags);
            let text_display_active = vector.text_active != u8::MIN;
            let clear_gate = !request_flags.any_request_pending()
                && (!text_display_active || vector.name == "active_honk_pair_clears_8010");
            let initial_flags = actor_base_flags.map(|flags| {
                if clear_gate {
                    flags | TRANSIENT_ACTOR_FLAGS
                } else {
                    flags
                }
            });
            let world_matches = vector.resolver_calls.len() == 2;
            let arche_matches = vector.name == "arche_fallback_match_sets_bit_10";
            let actor_position = POSITION_A;
            let world_position = if world_matches {
                POSITION_A
            } else {
                POSITION_B
            };
            let arche_position = if arche_matches {
                POSITION_A
            } else {
                POSITION_C
            };
            let leading_non_actor = vector.processed_entries.len() > 1;
            let mut fixture = fixture(
                initial_flags,
                actor_position,
                world_position,
                arche_position,
                leading_non_actor,
            );
            let honk_pair = (vector.name == "active_honk_pair_clears_8010")
                .then(|| fixture.actor.expect("Honk-pair vector has an actor"));
            let post_update = if vector.name == "active_honk_mismatch_preserves" {
                Some(fixture.world)
            } else {
                honk_pair
            };

            let outcome = update_actor_position_states(
                &mut fixture.state,
                ActorPositionStateContext {
                    request_flags,
                    text_display_active,
                    honk: honk_pair.or(fixture.actor.filter(|_| text_display_active)),
                    post_update,
                    world: fixture.world,
                    arche: fixture.arche,
                },
            )
            .unwrap();

            if let Some(actor) = fixture.actor {
                assert_eq!(outcome.processed_actors, 1, "{}", vector.name);
                assert_eq!(
                    actor_flags(&fixture.state, actor),
                    expected_flags,
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(outcome.processed_actors, usize::MIN, "{}", vector.name);
            }
        }
    }

    #[test]
    fn every_shipped_profile_normalizes_without_changing_non_actor_records() {
        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let mut state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();
            let before = state.clone();
            let world = directory.find_active_object(BUILTIN_WORLD_NAME).unwrap();
            let arche = directory
                .find_active_object(BUILTIN_ARCHETYPE_NAME)
                .unwrap();
            let honk = directory.find_active_object(BUILTIN_HONK_NAME);

            let outcome = update_actor_position_states(
                &mut state,
                ActorPositionStateContext {
                    request_flags: PresentationRequestFlags::default(),
                    text_display_active: false,
                    honk,
                    post_update: None,
                    world,
                    arche,
                },
            )
            .unwrap();
            assert_eq!(
                outcome.processed_actors,
                before
                    .objects()
                    .iter()
                    .filter(|object| object.kind == ScriptObjectKind::Actor)
                    .count()
            );
            for object in before
                .objects()
                .iter()
                .filter(|object| object.kind != ScriptObjectKind::Actor)
            {
                assert_eq!(state.object(object.id).unwrap().bytes(), object.bytes());
            }
        }
    }

    #[test]
    fn failed_navigation_is_transactional() {
        let mut fixture = fixture(Some(u16::MAX), POSITION_A, POSITION_B, POSITION_C, false);
        let actor = fixture.actor.unwrap();
        let holder_offset = script_field_offset(
            ScriptObjectKind::Actor,
            ScriptFieldSelector::HOLDER_OR_LOCATION,
        )
        .unwrap();
        let holder = fixture
            .state
            .object_word(actor, holder_offset / SERIALIZED_WORD_SIZE)
            .unwrap();
        assert!(fixture.state.set_word(holder, u16::MIN));
        let before = fixture.state.clone();

        assert!(matches!(
            update_actor_position_states(
                &mut fixture.state,
                ActorPositionStateContext {
                    request_flags: PresentationRequestFlags::default(),
                    text_display_active: false,
                    honk: None,
                    post_update: None,
                    world: fixture.world,
                    arche: fixture.arche,
                },
            ),
            Err(ActorPositionStateError::Navigation(_))
        ));
        assert_eq!(fixture.state, before);
    }
}
