//! Canonical VAR-backed state for translated BloodScript record handlers.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::mem::size_of;

use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptDirectRecordOperation, ScriptRecordStateOperand,
    ScriptRecordValue,
};
use commander_blood_formats::script::{
    ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptState, ScriptStateWord,
    ScriptStateWordTriple,
};

use super::super::{
    ScriptActionRecord, ScriptActionRecords, ScriptFieldSelector, ScriptRecordFields,
    ScriptRecordRuntime, ScriptTransferRecord, ScriptTransferRecords, insert_aboard_object,
    script_field_offset,
};
use super::ScriptProfileBuiltins;

const RECORD_KIND_NAVIGATION: u16 = 0x00C1;
const RECORD_KIND_ABOARD: u16 = 0x00C2;
const RECORD_KIND_PRESENTATION_QUEUE: u16 = 0x00C3;
const RECORD_KIND_WORLD_STATE: u16 = 0x00C5;
const RECORD_KIND_TRAVEL: u16 = 0x00C6;
const RECORD_KIND_ACTIVE_OBJECT: u16 = 0x00C7;
const RECORD_KIND_OPAQUE_MARKER: u16 = 0x00C8;
const RECORD_KIND_TRANSFER: u16 = 0x00CD;
const NAVIGATION_AUX_WORD: u16 = 2;
const PRESENTATION_QUEUE_AUX_WORD: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordFieldDomain {
    Relationship,
    Topic,
}

/// Invalid or internally inconsistent typed state recovered from one VAR image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptProfileRecordStateError {
    detail: ScriptProfileRecordStateErrorDetail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScriptProfileRecordStateErrorDetail {
    MissingPlayerBinding,
    AmbiguousFieldDomain { field: ScriptStateWord },
    MissingField { field: ScriptStateWord },
    MissingRecord { record: ScriptStateWordTriple },
    MissingObjectEncoding { object: ScriptObjectId },
    MissingDictionaryEncoding,
    AboardRosterOverflow { object: ScriptObjectId },
}

impl ScriptProfileRecordStateError {
    fn new(detail: ScriptProfileRecordStateErrorDetail) -> Self {
        Self { detail }
    }
}

impl fmt::Display for ScriptProfileRecordStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid profile record state: {:?}", self.detail)
    }
}

impl std::error::Error for ScriptProfileRecordStateError {}

/// Typed handler stores recovered from, and serializable back into, one VAR image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptProfileRecordState {
    /// Action triples consumed by C1-C9 and post-frame dispatch.
    pub action_records: ScriptActionRecords,
    /// Relationship and topic words consumed by the direct-record family.
    pub record_fields: ScriptRecordFields,
    /// CD triples used by transfer query mode.
    pub transfer_records: ScriptTransferRecords,
    /// Aboard roster and published direct-record value.
    pub record_runtime: ScriptRecordRuntime,
    action_slots: BTreeSet<ScriptStateWordTriple>,
    field_domains: BTreeMap<ScriptStateWord, RecordFieldDomain>,
}

impl ScriptProfileRecordState {
    pub(in crate::native::bloodprg) fn recover(
        instructions: &[DecodedScriptInstruction],
        state: &ScriptState,
        dictionary: &ScriptDictionary,
        builtins: ScriptProfileBuiltins,
    ) -> Result<Self, ScriptProfileRecordStateError> {
        Self::recover_with_roster(instructions, state, dictionary, builtins, None)
    }

    fn recover_with_roster(
        instructions: &[DecodedScriptInstruction],
        state: &ScriptState,
        dictionary: &ScriptDictionary,
        builtins: ScriptProfileBuiltins,
        roster: Option<&super::super::AboardObjectRoster>,
    ) -> Result<Self, ScriptProfileRecordStateError> {
        let player = builtins.player.ok_or_else(|| {
            ScriptProfileRecordStateError::new(
                ScriptProfileRecordStateErrorDetail::MissingPlayerBinding,
            )
        })?;
        let mut action_slots = BTreeSet::new();
        let mut field_domains = BTreeMap::new();
        let mut transfer_sources = BTreeSet::new();

        for object in state.objects() {
            if let Some(offset) = script_field_offset(object.kind, ScriptFieldSelector::ACTION)
                && let Some(slot) = state.object_word_triple(object.id, offset / size_of::<u16>())
            {
                action_slots.insert(slot);
            }
            if let Some(offset) =
                script_field_offset(object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
                && let Some(field) = state.object_word(object.id, offset / size_of::<u16>())
            {
                insert_field_domain(&mut field_domains, field, RecordFieldDomain::Relationship)?;
            }
        }

        for instruction in instructions {
            match instruction {
                DecodedScriptInstruction::DirectRecord(operation) => {
                    insert_field_domain(
                        &mut field_domains,
                        operation.target,
                        direct_record_domain(*operation),
                    )?;
                }
                DecodedScriptInstruction::RecordState(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::AboardRecord(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::PresentationQueue(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::ActorRecord(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::WorldStateRecord(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::TravelRecord(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::ActiveObjectRecord(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::OpaqueMarkerRecord(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::RecordClear(operation) => {
                    action_slots.insert(operation.target);
                }
                DecodedScriptInstruction::Transfer(transfer) => {
                    transfer_sources.insert(transfer.source_record);
                }
                _ => {}
            }
        }

        let mut action_records = ScriptActionRecords::default();
        for slot in &action_slots {
            let raw = state.word_triple(*slot).ok_or_else(|| {
                ScriptProfileRecordStateError::new(
                    ScriptProfileRecordStateErrorDetail::MissingRecord { record: *slot },
                )
            })?;
            action_records.set_record(*slot, decode_action_record(raw, state));
            if raw[0] != u16::MIN && raw[2] == u16::MAX {
                action_records.set_actionable(*slot, false);
            }
        }

        let mut record_fields = ScriptRecordFields::default();
        let mut record_runtime = ScriptRecordRuntime::new(player);
        for (field, domain) in &field_domains {
            let raw = state.word(*field).ok_or_else(|| {
                ScriptProfileRecordStateError::new(
                    ScriptProfileRecordStateErrorDetail::MissingField { field: *field },
                )
            })?;
            record_fields.set_value(*field, decode_record_value(raw, *domain, state, dictionary));
            if roster.is_none()
                && raw == u16::MAX
                && let Some(object) = field.object()
                && !insert_aboard_object(record_runtime.aboard_objects_mut(), object)
            {
                return Err(ScriptProfileRecordStateError::new(
                    ScriptProfileRecordStateErrorDetail::AboardRosterOverflow { object },
                ));
            }
        }

        if let Some(roster) = roster {
            *record_runtime.aboard_objects_mut() = roster.clone();
        }

        let mut transfer_records = ScriptTransferRecords::default();
        for source in transfer_sources {
            let Some(record) = state.word_triple_starting_at(source) else {
                continue;
            };
            let Some(raw) = state.word_triple(record) else {
                continue;
            };
            if raw[0] == RECORD_KIND_TRANSFER
                && let (Some(item), Some(destination)) =
                    (decode_object(raw[1], state), decode_object(raw[2], state))
            {
                transfer_records.set_record(source, ScriptTransferRecord { item, destination });
            }
        }

        Ok(Self {
            action_records,
            record_fields,
            transfer_records,
            record_runtime,
            action_slots,
            field_domains,
        })
    }

    pub(super) fn synchronize_into(
        &self,
        state: &mut ScriptState,
        directory: &ScriptDirectory,
        dictionary: &ScriptDictionary,
    ) -> Result<(), ScriptProfileRecordStateError> {
        for slot in &self.action_slots {
            let record = self.action_records.record(*slot);
            let Some(mut raw) = encode_action_record(record, directory)? else {
                continue;
            };
            if record != ScriptActionRecord::Empty && !self.action_records.is_actionable(*slot) {
                raw[2] = u16::MAX;
            }
            if !state.set_word_triple(*slot, raw) {
                return Err(ScriptProfileRecordStateError::new(
                    ScriptProfileRecordStateErrorDetail::MissingRecord { record: *slot },
                ));
            }
        }

        for field in self.field_domains.keys() {
            let value = self.record_fields.value(*field).ok_or_else(|| {
                ScriptProfileRecordStateError::new(
                    ScriptProfileRecordStateErrorDetail::MissingField { field: *field },
                )
            })?;
            let raw = encode_record_value(value, directory, dictionary)?;
            if !state.set_word(*field, raw) {
                return Err(ScriptProfileRecordStateError::new(
                    ScriptProfileRecordStateErrorDetail::MissingField { field: *field },
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn refresh_from_var(
        &mut self,
        instructions: &[DecodedScriptInstruction],
        state: &ScriptState,
        dictionary: &ScriptDictionary,
        builtins: ScriptProfileBuiltins,
    ) -> Result<(), ScriptProfileRecordStateError> {
        let recovered = self.recover_for_refresh(instructions, state, dictionary, builtins)?;
        self.action_records = recovered.action_records.clone();
        self.transfer_records = recovered.transfer_records.clone();
        self.replace_record_fields_and_roster(&recovered);
        Ok(())
    }

    pub(crate) fn refresh_relationships_from_var(
        &mut self,
        instructions: &[DecodedScriptInstruction],
        state: &ScriptState,
        dictionary: &ScriptDictionary,
        builtins: ScriptProfileBuiltins,
    ) -> Result<(), ScriptProfileRecordStateError> {
        let recovered = self.recover_for_refresh(instructions, state, dictionary, builtins)?;
        self.transfer_records = recovered.transfer_records.clone();
        self.replace_record_fields_and_roster(&recovered);
        Ok(())
    }

    pub(crate) fn commit_to_var(
        &self,
        state: &mut ScriptState,
        directory: &ScriptDirectory,
        dictionary: &ScriptDictionary,
    ) -> Result<(), ScriptProfileRecordStateError> {
        self.synchronize_into(state, directory, dictionary)
    }

    fn recover_for_refresh(
        &self,
        instructions: &[DecodedScriptInstruction],
        state: &ScriptState,
        dictionary: &ScriptDictionary,
        builtins: ScriptProfileBuiltins,
    ) -> Result<Self, ScriptProfileRecordStateError> {
        // BLOOD2 6038 refreshes actor fields; 59FF rebuilds the roster only at load.
        let roster = (state.dialect() == commander_blood_formats::code::ScriptDialect::BigBugBang)
            .then(|| self.record_runtime.aboard_objects());
        Self::recover_with_roster(instructions, state, dictionary, builtins, roster)
    }

    fn replace_record_fields_and_roster(&mut self, recovered: &Self) {
        self.record_fields = recovered.record_fields.clone();
        *self.record_runtime.aboard_objects_mut() =
            recovered.record_runtime.aboard_objects().clone();
    }
}

fn insert_field_domain(
    domains: &mut BTreeMap<ScriptStateWord, RecordFieldDomain>,
    field: ScriptStateWord,
    domain: RecordFieldDomain,
) -> Result<(), ScriptProfileRecordStateError> {
    if let Some(previous) = domains.insert(field, domain)
        && previous != domain
    {
        return Err(ScriptProfileRecordStateError::new(
            ScriptProfileRecordStateErrorDetail::AmbiguousFieldDomain { field },
        ));
    }
    Ok(())
}

const fn direct_record_domain(operation: ScriptDirectRecordOperation) -> RecordFieldDomain {
    if operation.publishes_value {
        RecordFieldDomain::Topic
    } else {
        RecordFieldDomain::Relationship
    }
}

fn decode_record_value(
    raw: u16,
    domain: RecordFieldDomain,
    state: &ScriptState,
    dictionary: &ScriptDictionary,
) -> ScriptRecordValue {
    if raw == u16::MAX {
        return ScriptRecordValue::Aboard;
    }
    match domain {
        RecordFieldDomain::Relationship => decode_object(raw, state)
            .map(ScriptRecordValue::Object)
            .unwrap_or(ScriptRecordValue::NativeWord(raw)),
        RecordFieldDomain::Topic => dictionary
            .resolve_source_offset(raw)
            .map(ScriptRecordValue::Topic)
            .unwrap_or(ScriptRecordValue::NativeWord(raw)),
    }
}

fn encode_record_value(
    value: ScriptRecordValue,
    directory: &ScriptDirectory,
    dictionary: &ScriptDictionary,
) -> Result<u16, ScriptProfileRecordStateError> {
    match value {
        ScriptRecordValue::Object(object) => encode_object(object, directory),
        ScriptRecordValue::Aboard => Ok(u16::MAX),
        ScriptRecordValue::Topic(word) => dictionary.source_offset(word).ok_or_else(|| {
            ScriptProfileRecordStateError::new(
                ScriptProfileRecordStateErrorDetail::MissingDictionaryEncoding,
            )
        }),
        ScriptRecordValue::NativeWord(raw) => Ok(raw),
    }
}

fn decode_action_record(raw: [u16; 3], state: &ScriptState) -> ScriptActionRecord {
    match raw[0] {
        0 => ScriptActionRecord::Empty,
        RECORD_KIND_NAVIGATION => ScriptActionRecord::Navigation(match raw[1] {
            1 => ScriptRecordStateOperand::PrimaryNavigationObject,
            2 => ScriptRecordStateOperand::SecondaryNavigationObject,
            encoded => decode_object(encoded, state)
                .map(ScriptRecordStateOperand::Object)
                .unwrap_or(ScriptRecordStateOperand::NativeWord(encoded)),
        }),
        RECORD_KIND_ABOARD => decode_object(raw[1], state)
            .map(ScriptActionRecord::AboardRequest)
            .unwrap_or(ScriptActionRecord::Occupied),
        RECORD_KIND_PRESENTATION_QUEUE => decode_object(raw[1], state)
            .map(ScriptActionRecord::PresentationQueue)
            .unwrap_or(ScriptActionRecord::Occupied),
        ScriptActionRecord::ACTOR_PRESENTATION_KIND => decode_object(raw[1], state)
            .map(ScriptActionRecord::ActorPresentation)
            .unwrap_or(ScriptActionRecord::Occupied),
        RECORD_KIND_WORLD_STATE => decode_object(raw[1], state)
            .map(ScriptActionRecord::WorldStateLink)
            .unwrap_or(ScriptActionRecord::Occupied),
        RECORD_KIND_TRAVEL => decode_object(raw[1], state)
            .map(ScriptActionRecord::Travel)
            .unwrap_or(ScriptActionRecord::Occupied),
        RECORD_KIND_ACTIVE_OBJECT => decode_object(raw[1], state)
            .map(ScriptActionRecord::ActiveObjectLink)
            .unwrap_or(ScriptActionRecord::Occupied),
        RECORD_KIND_OPAQUE_MARKER => ScriptActionRecord::OpaqueMarker(raw[1]),
        _ => ScriptActionRecord::Occupied,
    }
}

fn encode_action_record(
    record: ScriptActionRecord,
    directory: &ScriptDirectory,
) -> Result<Option<[u16; 3]>, ScriptProfileRecordStateError> {
    let raw = match record {
        ScriptActionRecord::Empty => [u16::MIN; 3],
        ScriptActionRecord::Navigation(operand) => [
            RECORD_KIND_NAVIGATION,
            encode_navigation_operand(operand, directory)?,
            NAVIGATION_AUX_WORD,
        ],
        ScriptActionRecord::AboardRequest(related) => {
            [RECORD_KIND_ABOARD, encode_object(related, directory)?, 0]
        }
        ScriptActionRecord::PresentationQueue(related) => [
            RECORD_KIND_PRESENTATION_QUEUE,
            encode_object(related, directory)?,
            PRESENTATION_QUEUE_AUX_WORD,
        ],
        ScriptActionRecord::ActorPresentation(related) => [
            ScriptActionRecord::ACTOR_PRESENTATION_KIND,
            encode_object(related, directory)?,
            0,
        ],
        ScriptActionRecord::WorldStateLink(related) => [
            RECORD_KIND_WORLD_STATE,
            encode_object(related, directory)?,
            0,
        ],
        ScriptActionRecord::Travel(destination) => [
            RECORD_KIND_TRAVEL,
            encode_object(destination, directory)?,
            0,
        ],
        ScriptActionRecord::ActiveObjectLink(related) => [
            RECORD_KIND_ACTIVE_OBJECT,
            encode_object(related, directory)?,
            0,
        ],
        ScriptActionRecord::OpaqueMarker(word) => [RECORD_KIND_OPAQUE_MARKER, word, 0],
        ScriptActionRecord::Occupied => return Ok(None),
    };
    Ok(Some(raw))
}

fn encode_navigation_operand(
    operand: ScriptRecordStateOperand,
    directory: &ScriptDirectory,
) -> Result<u16, ScriptProfileRecordStateError> {
    match operand {
        ScriptRecordStateOperand::PrimaryNavigationObject => Ok(1),
        ScriptRecordStateOperand::SecondaryNavigationObject => Ok(2),
        ScriptRecordStateOperand::Object(object) => encode_object(object, directory),
        ScriptRecordStateOperand::NativeWord(raw) => Ok(raw),
    }
}

fn decode_object(encoded: u16, state: &ScriptState) -> Option<ScriptObjectId> {
    state
        .objects()
        .iter()
        .find_map(|object| (object.source_offset() == usize::from(encoded)).then_some(object.id))
}

fn encode_object(
    object: ScriptObjectId,
    directory: &ScriptDirectory,
) -> Result<u16, ScriptProfileRecordStateError> {
    directory
        .object(object)
        .map(|entry| entry.value)
        .ok_or_else(|| {
            ScriptProfileRecordStateError::new(
                ScriptProfileRecordStateErrorDetail::MissingObjectEncoding { object },
            )
        })
}
