//! Typed relationship and topic fields used by BloodScript record operations.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::instruction::{
    ScriptDirectRecordOperation, ScriptRecordPairOperation, ScriptRecordValue, ScriptTransfer,
};
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateWord, ScriptStateWordPair,
    ScriptWordId,
};

use super::{
    AboardObjectRoster, PresentationRequestFlags, ScriptControl, ScriptFieldSelector,
    ScriptRuntime, ScriptRuntimeError, insert_aboard_object, remove_aboard_object,
    script_field_offset,
};

/// Owned typed values for relationship and topic fields reached by direct-record tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptRecordFields {
    values: BTreeMap<ScriptStateWord, ScriptRecordValue>,
}

/// One typed CD record triple retained for query-mode comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptTransferRecord {
    /// Object moved by the transfer.
    pub item: ScriptObjectId,
    /// Destination object encoded by the transfer record.
    pub destination: ScriptObjectId,
}

/// Owned CD record triples keyed by their typed state-field identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptTransferRecords {
    records: BTreeMap<ScriptStateWord, ScriptTransferRecord>,
}

/// Typed owner reference invalidated when its object's adjacent-word record changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptRecordPairReference {
    object: Option<ScriptObjectId>,
}

impl ScriptRecordPairReference {
    /// Construct the owner reference held by the native active-presentation record.
    pub const fn new(object: Option<ScriptObjectId>) -> Self {
        Self { object }
    }

    /// Return the currently referenced object.
    pub const fn object(self) -> Option<ScriptObjectId> {
        self.object
    }
}

impl ScriptTransferRecords {
    /// Read one transfer record, or `None` when another record kind occupies it.
    pub fn record(&self, field: ScriptStateWord) -> Option<ScriptTransferRecord> {
        self.records.get(&field).copied()
    }

    /// Store one typed transfer record for later query-mode evaluation.
    pub fn set_record(&mut self, field: ScriptStateWord, record: ScriptTransferRecord) {
        self.records.insert(field, record);
    }

    /// Remove any transfer record at the selected field.
    pub fn clear_record(&mut self, field: ScriptStateWord) {
        self.records.remove(&field);
    }
}

impl ScriptRecordFields {
    /// Read one typed record field.
    pub fn value(&self, field: ScriptStateWord) -> Option<ScriptRecordValue> {
        self.values.get(&field).copied()
    }

    /// Initialize or replace one typed record field.
    pub fn set_value(&mut self, field: ScriptStateWord, value: ScriptRecordValue) {
        self.values.insert(field, value);
    }
}

/// Shared typed state used by the direct-record handler family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptRecordRuntime {
    special_object: ScriptObjectId,
    aboard_objects: AboardObjectRoster,
    published_value: Option<ScriptRecordValue>,
}

impl ScriptRecordRuntime {
    /// Construct record state for the profile's built-in player object.
    pub fn new(special_object: ScriptObjectId) -> Self {
        Self {
            special_object,
            aboard_objects: AboardObjectRoster::default(),
            published_value: None,
        }
    }

    /// Return objects tracked as aboard through sentinel-valued fields.
    pub fn aboard_objects(&self) -> &AboardObjectRoster {
        &self.aboard_objects
    }

    /// Return mutable aboard state for save restoration and related handlers.
    pub fn aboard_objects_mut(&mut self) -> &mut AboardObjectRoster {
        &mut self.aboard_objects
    }

    /// Return the most recent BC publication in its typed value domain.
    pub const fn published_value(&self) -> Option<ScriptRecordValue> {
        self.published_value
    }

    /// Return the most recent BC publication when it is an interned topic.
    pub const fn published_topic(&self) -> Option<ScriptWordId> {
        match self.published_value {
            Some(ScriptRecordValue::Topic(topic)) => Some(topic),
            _ => None,
        }
    }
}

/// Presentation line selected after a descriptor-backed inventory transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptTransferPresentationLine {
    /// Native active-line value 43, selected for a moved inventory item.
    InventoryMoved,
}

impl ScriptTransferPresentationLine {
    /// Return the shared native `vm_active_line` value written by CD.
    pub const fn number(self) -> u16 {
        match self {
            Self::InventoryMoved => 43,
        }
    }
}

/// Presentation state changed by a successful descriptor-backed transfer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptTransferPresentationState {
    /// Existing presentation work still owns the descriptor path.
    pub presentation_gate_active: bool,
    /// Presentation line requested by the transfer handler.
    pub active_line: Option<ScriptTransferPresentationLine>,
}

/// Already-resolved host and descriptor inputs used by transfer presentation gates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptTransferContext {
    /// The ship interface currently blocks transfer presentation work.
    pub ship_interface_active: bool,
    /// The moved object's name has a matching descriptor entry.
    pub descriptor_available: bool,
}

/// Observable effects of one CD handler invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptTransferOutcome {
    /// Script control flow after a query or assignment.
    pub control: ScriptControl,
    /// Whether the moved object's holder changed.
    pub holder_changed: bool,
    /// Whether the descriptor catalog was consulted after all earlier gates.
    pub descriptor_checked: bool,
    /// Whether descriptor-backed presentation work was requested.
    pub presentation_requested: bool,
}

/// Invalid typed state or control flow in one direct-record operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRecordError {
    /// The operation's typed target has not been initialized.
    MissingField {
        /// Missing field identity.
        field: ScriptStateWord,
    },
    /// An adjacent-word field belongs to a different or truncated profile state.
    MissingPair {
        /// Missing pair identity.
        pair: ScriptStateWordPair,
    },
    /// An aboard transition targets state not owned by a profile object.
    MissingOwner {
        /// Ownerless field identity.
        field: ScriptStateWord,
    },
    /// The moved object is absent from the decoded profile state.
    MissingObject {
        /// Missing object identity.
        object: ScriptObjectId,
    },
    /// The moved object's kind has no proven holder field.
    MissingHolderField {
        /// Object whose holder field could not be resolved.
        object: ScriptObjectId,
    },
    /// A failed query had no procedure or nested guard destination.
    Control(ScriptRuntimeError),
}

impl fmt::Display for ScriptRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptRecordError {}

/// Apply `vm_op_shared_record_wildcard` to typed relationships and topics.
pub fn apply_direct_record_operation(
    operation: ScriptDirectRecordOperation,
    fields: &mut ScriptRecordFields,
    record_runtime: &mut ScriptRecordRuntime,
    script_runtime: &mut ScriptRuntime,
) -> Result<ScriptControl, ScriptRecordError> {
    let current = fields
        .value(operation.target)
        .ok_or(ScriptRecordError::MissingField {
            field: operation.target,
        })?;

    if script_runtime.query_mode() {
        let expected =
            if operation.value == ScriptRecordValue::Object(record_runtime.special_object) {
                ScriptRecordValue::Aboard
            } else {
                operation.value
            };
        if (current == expected) != operation.inverted {
            return Ok(ScriptControl::Continue);
        }
        return script_runtime
            .fail_guard()
            .map_err(ScriptRecordError::Control);
    }

    if operation.publishes_value {
        record_runtime.published_value = Some(operation.value);
    }
    if current == ScriptRecordValue::Aboard {
        let owner = operation
            .target
            .object()
            .ok_or(ScriptRecordError::MissingOwner {
                field: operation.target,
            })?;
        remove_aboard_object(&mut record_runtime.aboard_objects, owner);
        fields.set_value(operation.target, operation.value);
        return Ok(ScriptControl::Continue);
    }

    let requests_aboard = operation.value == ScriptRecordValue::Aboard
        || operation.value == ScriptRecordValue::Object(record_runtime.special_object);
    let stored_value = if requests_aboard {
        let owner = operation
            .target
            .object()
            .ok_or(ScriptRecordError::MissingOwner {
                field: operation.target,
            })?;
        if !insert_aboard_object(&mut record_runtime.aboard_objects, owner) {
            return Ok(ScriptControl::Continue);
        }
        ScriptRecordValue::Aboard
    } else {
        operation.value
    };
    fields.set_value(operation.target, stored_value);
    Ok(ScriptControl::Continue)
}

/// Apply `vm_op_b8_record_readwrite` to a bounded pair and typed owner reference.
pub fn apply_record_pair_operation(
    operation: ScriptRecordPairOperation,
    state: &mut ScriptState,
    reference: &mut ScriptRecordPairReference,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptControl, ScriptRecordError> {
    let current = state
        .word_pair(operation.target)
        .ok_or(ScriptRecordError::MissingPair {
            pair: operation.target,
        })?;
    if runtime.query_mode() {
        if current == operation.value {
            return Ok(ScriptControl::Continue);
        }
        return runtime.fail_guard().map_err(ScriptRecordError::Control);
    }

    if !state.set_word_pair(operation.target, operation.value) {
        return Err(ScriptRecordError::MissingPair {
            pair: operation.target,
        });
    }
    if reference.object == operation.target.object() {
        reference.object = None;
    }
    Ok(ScriptControl::Continue)
}

/// Apply `vm_op_cd_state_gated` as a typed object transfer or record query.
#[allow(clippy::too_many_arguments)]
pub fn apply_transfer(
    transfer: ScriptTransfer,
    profile: &ScriptState,
    transfer_records: &ScriptTransferRecords,
    fields: &mut ScriptRecordFields,
    record_runtime: &mut ScriptRecordRuntime,
    context: ScriptTransferContext,
    request_flags: &mut PresentationRequestFlags,
    presentation: &mut ScriptTransferPresentationState,
    script_runtime: &mut ScriptRuntime,
) -> Result<ScriptTransferOutcome, ScriptRecordError> {
    if script_runtime.query_mode() {
        let expected = ScriptTransferRecord {
            item: transfer.item,
            destination: transfer.destination,
        };
        let matches = transfer_records.record(transfer.source_record) == Some(expected);
        let control = if matches != transfer.inverted {
            ScriptControl::Continue
        } else {
            script_runtime
                .fail_guard()
                .map_err(ScriptRecordError::Control)?
        };
        return Ok(ScriptTransferOutcome {
            control,
            holder_changed: false,
            descriptor_checked: false,
            presentation_requested: false,
        });
    }

    let source = transfer
        .source_record
        .object()
        .ok_or(ScriptRecordError::MissingOwner {
            field: transfer.source_record,
        })?;
    let item = profile
        .object(transfer.item)
        .ok_or(ScriptRecordError::MissingObject {
            object: transfer.item,
        })?;
    let holder_byte_offset =
        script_field_offset(item.kind, ScriptFieldSelector::HOLDER_OR_LOCATION).ok_or(
            ScriptRecordError::MissingHolderField {
                object: transfer.item,
            },
        )?;
    let holder_field = profile
        .object_word(transfer.item, holder_byte_offset / size_of::<u16>())
        .ok_or(ScriptRecordError::MissingHolderField {
            object: transfer.item,
        })?;

    if source == record_runtime.special_object {
        remove_aboard_object(&mut record_runtime.aboard_objects, transfer.item);
    }
    let destination_is_aboard = transfer.destination == record_runtime.special_object;
    if destination_is_aboard
        && !insert_aboard_object(&mut record_runtime.aboard_objects, transfer.item)
    {
        return Ok(ScriptTransferOutcome {
            control: ScriptControl::Continue,
            holder_changed: false,
            descriptor_checked: false,
            presentation_requested: false,
        });
    }
    fields.set_value(
        holder_field,
        if destination_is_aboard {
            ScriptRecordValue::Aboard
        } else {
            ScriptRecordValue::Object(transfer.destination)
        },
    );

    let can_check_descriptor = !context.ship_interface_active
        && !request_flags.secondary_request_pending()
        && item.kind == ScriptObjectKind::InventoryItem;
    let presentation_requested = can_check_descriptor && context.descriptor_available;
    if presentation_requested {
        presentation.presentation_gate_active = false;
        request_flags.request_secondary();
        presentation.active_line = Some(ScriptTransferPresentationLine::InventoryMoved);
    }

    Ok(ScriptTransferOutcome {
        control: ScriptControl::Continue,
        holder_changed: true,
        descriptor_checked: can_check_descriptor,
        presentation_requested,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::{ScriptCodeOffset, decode_script_code};
    use commander_blood_formats::instruction::decode_script_record_pair_operation;
    use commander_blood_formats::script::{
        ScriptDirectory, ScriptState, decode_script_directory, decode_script_state,
    };
    use serde::Deserialize;

    use super::*;

    const DIRECT_RECORD_VECTOR_COUNT: usize = 17;
    const TRANSFER_VECTOR_COUNT: usize = 20;
    const RECORD_PAIR_VECTOR_COUNT: usize = 10;
    const QUERY_MODE_MASK: u8 = 1;
    const TOPIC_PUBLICATION_OPCODE: u8 = 0xBC;
    const SECONDARY_PRESENTATION_REQUEST_BIT: u8 = 2;
    const BRANCH_TARGET: usize = 9_320;
    const TEST_FIELD_WORD_INDEX: usize = 1;
    const PAIR_RECORD_OPCODE: u8 = 0xBD;
    const END_MARKER: u8 = 0xFF;
    const POSITION_BYTE_OFFSET: u16 = 24;

    #[derive(Deserialize)]
    struct DirectRecordOracle {
        name: String,
        query_mode_before: u8,
        query_mode_after: u8,
        inverted: bool,
        opcode: u8,
        field_before: u16,
        requested_value: u16,
        wildcard_value: u16,
        field_after: u16,
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct TransferOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        third_record: u16,
        kind: Option<u16>,
        field_before: Option<u16>,
        field_after: Option<u16>,
        remove_called: bool,
        insert_called: bool,
        insert_success: bool,
        c2_called: bool,
        c2_result: Option<u16>,
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct RecordPairOracle {
        name: String,
        query_mode_before: u8,
        requested_pair: [u16; 2],
        pair_before: [u16; 2],
        pair_after: [u16; 2],
        owner: u16,
        secondary_link_before: u16,
        secondary_link_after: u16,
        branch_failed: bool,
        query_mode_after: u8,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn record_pairs_match_every_original_handler_vector() {
        let vectors: Vec<RecordPairOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6b06_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RECORD_PAIR_VECTOR_COUNT);
        let directory_data = std::fs::read(original_asset("SCRIPT3.DEB")).unwrap();
        let state_data = std::fs::read(original_asset("SCRIPT3.VAR")).unwrap();
        let directory = decode_script_directory(&directory_data).unwrap();
        let owner = directory.find_active_object(b"Kraner").unwrap();
        let other = directory.find_active_object(b"blood").unwrap();
        let owner_offset = directory.object(owner).unwrap().value;

        for vector in vectors {
            let target_offset = owner_offset.wrapping_add(POSITION_BYTE_OFFSET);
            let mut token_data = vec![PAIR_RECORD_OPCODE];
            token_data.extend_from_slice(&target_offset.to_le_bytes());
            token_data.extend_from_slice(&vector.requested_pair[0].to_le_bytes());
            token_data.extend_from_slice(&vector.requested_pair[1].to_le_bytes());
            token_data.push(END_MARKER);
            let code = decode_script_code(&token_data).unwrap();
            let mut state = decode_script_state(&state_data, &directory).unwrap();
            let operation = decode_script_record_pair_operation(&code.tokens()[0], &state).unwrap();
            assert_eq!(operation.target.object(), Some(owner), "{}", vector.name);
            assert!(state.set_word_pair(operation.target, vector.pair_before));
            let initial_reference = if vector.secondary_link_before == u16::MIN {
                None
            } else if vector.secondary_link_before == vector.owner {
                Some(owner)
            } else {
                Some(other)
            };
            let mut reference = ScriptRecordPairReference::new(initial_reference);
            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_MASK != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
            }

            let control =
                apply_record_pair_operation(operation, &mut state, &mut reference, &mut runtime)
                    .unwrap();

            assert_eq!(
                state.word_pair(operation.target),
                Some(vector.pair_after),
                "{}",
                vector.name
            );
            let expected_reference = if vector.secondary_link_after == u16::MIN {
                None
            } else if vector.secondary_link_after == vector.owner {
                Some(owner)
            } else {
                Some(other)
            };
            assert_eq!(reference.object(), expected_reference, "{}", vector.name);
            assert_eq!(
                runtime.query_mode(),
                vector.query_mode_after & QUERY_MODE_MASK != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
        }
    }

    fn profile(number: usize) -> (ScriptDirectory, ScriptState) {
        let directory = decode_script_directory(
            &std::fs::read(original_asset(&format!("SCRIPT{number}.DEB"))).unwrap(),
        )
        .unwrap();
        let state = decode_script_state(
            &std::fs::read(original_asset(&format!("SCRIPT{number}.VAR"))).unwrap(),
            &directory,
        )
        .unwrap();
        (directory, state)
    }

    fn object_of_kind(state: &ScriptState, kind: ScriptObjectKind) -> ScriptObjectId {
        state
            .objects()
            .iter()
            .find_map(|object| (object.kind == kind).then_some(object.id))
            .unwrap()
    }

    fn holder_field(state: &ScriptState, object: ScriptObjectId) -> ScriptStateWord {
        let state_object = state.object(object).unwrap();
        let byte_offset =
            script_field_offset(state_object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
                .unwrap();
        state
            .object_word(object, byte_offset / size_of::<u16>())
            .unwrap()
    }

    fn oracle_value(
        value: u16,
        wildcard_value: u16,
        special_object: ScriptObjectId,
    ) -> ScriptRecordValue {
        if value == u16::MAX {
            ScriptRecordValue::Aboard
        } else if value == wildcard_value {
            ScriptRecordValue::Object(special_object)
        } else {
            ScriptRecordValue::NativeWord(value)
        }
    }

    #[test]
    fn direct_records_match_every_original_handler_vector() {
        let vectors: Vec<DirectRecordOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6946_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), DIRECT_RECORD_VECTOR_COUNT);
        let (directory, state) = profile(1);
        let special_object = directory.find_active_object(b"blood").unwrap();
        let target_owner = directory.active_objects().nth(1).unwrap().0;
        let target = state
            .resolve_word_source_offset(
                (state.object(target_owner).unwrap().source_offset()
                    + TEST_FIELD_WORD_INDEX * size_of::<u16>()) as u16,
            )
            .unwrap();

        for vector in vectors {
            let requested = oracle_value(
                vector.requested_value,
                vector.wildcard_value,
                special_object,
            );
            let expected = oracle_value(vector.field_after, vector.wildcard_value, special_object);
            let mut fields = ScriptRecordFields::default();
            fields.set_value(
                target,
                oracle_value(vector.field_before, vector.wildcard_value, special_object),
            );
            let mut record_runtime = ScriptRecordRuntime::new(special_object);
            let owner = target.object().unwrap();
            if vector.name.contains("remove_present") || vector.name.contains("insert_existing") {
                assert!(insert_aboard_object(
                    record_runtime.aboard_objects_mut(),
                    owner
                ));
            } else if vector.name.contains("full") {
                for object in directory
                    .active_objects()
                    .map(|(object, _entry)| object)
                    .filter(|object| *object != owner && object.index() != usize::MIN)
                    .take(super::super::ABOARD_OBJECT_CAPACITY)
                {
                    assert!(insert_aboard_object(
                        record_runtime.aboard_objects_mut(),
                        object
                    ));
                }
                assert!(
                    record_runtime
                        .aboard_objects()
                        .slots()
                        .iter()
                        .all(Option::is_some)
                );
            }
            let mut script_runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_MASK != u8::MIN {
                script_runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
            }
            let operation = ScriptDirectRecordOperation {
                target,
                value: requested,
                inverted: vector.inverted,
                publishes_value: vector.opcode == TOPIC_PUBLICATION_OPCODE,
            };

            let control = apply_direct_record_operation(
                operation,
                &mut fields,
                &mut record_runtime,
                &mut script_runtime,
            )
            .unwrap();

            assert_eq!(fields.value(target), Some(expected), "{}", vector.name);
            assert_eq!(
                script_runtime.query_mode(),
                vector.query_mode_after & QUERY_MODE_MASK != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            assert_eq!(
                record_runtime.published_value(),
                (vector.opcode == TOPIC_PUBLICATION_OPCODE
                    && vector.query_mode_before & QUERY_MODE_MASK == u8::MIN)
                    .then_some(requested),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn transfers_match_every_original_handler_vector() {
        let vectors: Vec<TransferOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_69c7_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TRANSFER_VECTOR_COUNT);
        let (directory, state) = profile(2);
        let special_object = directory.find_active_object(b"blood").unwrap();
        let actor = object_of_kind(&state, ScriptObjectKind::Actor);
        let inventory_item = object_of_kind(&state, ScriptObjectKind::InventoryItem);
        let world_state = object_of_kind(&state, ScriptObjectKind::WorldState);
        let alternate_object = state
            .objects()
            .iter()
            .find_map(|object| {
                (object.id != actor && object.id != inventory_item && object.id != special_object)
                    .then_some(object.id)
            })
            .unwrap();

        for vector in vectors {
            let query_mode = vector.query_mode_before & QUERY_MODE_MASK != u8::MIN;
            let item = if query_mode || vector.kind == Some(ScriptObjectKind::InventoryItem.mask())
            {
                inventory_item
            } else {
                world_state
            };
            let source = if vector.remove_called {
                special_object
            } else {
                actor
            };
            let destination = if vector.insert_called {
                special_object
            } else {
                actor
            };
            let source_record = state.object_word(source, TEST_FIELD_WORD_INDEX).unwrap();
            let transfer = ScriptTransfer {
                source_record,
                item,
                destination,
                inverted: vector.inverted,
            };
            let mut records = ScriptTransferRecords::default();
            if query_mode && !vector.name.contains("kind_mismatch") {
                records.set_record(
                    source_record,
                    ScriptTransferRecord {
                        item: if vector.name.contains("second_mismatch") {
                            alternate_object
                        } else {
                            item
                        },
                        destination: if vector.name.contains("third_mismatch")
                            || vector.name.contains("inverted_mismatch")
                            || vector.name.contains("inverted_segment_end")
                        {
                            alternate_object
                        } else {
                            destination
                        },
                    },
                );
            }

            let field = holder_field(&state, item);
            let mut fields = ScriptRecordFields::default();
            if let Some(initial_value) = vector.field_before.or_else(|| {
                (vector.insert_called && !vector.insert_success)
                    .then_some(vector.field_after)
                    .flatten()
            }) {
                fields.set_value(field, ScriptRecordValue::NativeWord(initial_value));
            }
            let mut record_runtime = ScriptRecordRuntime::new(special_object);
            if vector.remove_called || vector.name.contains("insert_existing") {
                assert!(insert_aboard_object(
                    record_runtime.aboard_objects_mut(),
                    item
                ));
            } else if vector.name.contains("insert_full") {
                for object in directory
                    .active_objects()
                    .map(|(object, _entry)| object)
                    .filter(|object| *object != item && object.index() != usize::MIN)
                    .take(super::super::ABOARD_OBJECT_CAPACITY)
                {
                    assert!(insert_aboard_object(
                        record_runtime.aboard_objects_mut(),
                        object
                    ));
                }
            }
            let initial_request_flags = if vector.name == "set_request_gate" {
                SECONDARY_PRESENTATION_REQUEST_BIT
            } else {
                u8::MIN
            };
            let mut request_flags = PresentationRequestFlags::decode(initial_request_flags);
            let mut presentation = ScriptTransferPresentationState {
                presentation_gate_active: true,
                ..ScriptTransferPresentationState::default()
            };
            let context = ScriptTransferContext {
                ship_interface_active: vector.name == "set_ui_gate",
                descriptor_available: vector.c2_result == Some(1),
            };
            let mut script_runtime = ScriptRuntime::new();
            if query_mode {
                script_runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
            }

            let outcome = apply_transfer(
                transfer,
                &state,
                &records,
                &mut fields,
                &mut record_runtime,
                context,
                &mut request_flags,
                &mut presentation,
                &mut script_runtime,
            )
            .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.descriptor_checked, vector.c2_called,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.presentation_requested,
                vector.c2_result == Some(1),
                "{}",
                vector.name
            );
            if let Some(field_after) = vector.field_after {
                let expected = if field_after == u16::MAX {
                    ScriptRecordValue::Aboard
                } else if field_after == vector.third_record {
                    ScriptRecordValue::Object(destination)
                } else {
                    ScriptRecordValue::NativeWord(field_after)
                };
                assert_eq!(fields.value(field), Some(expected), "{}", vector.name);
                assert_eq!(
                    outcome.holder_changed,
                    vector.insert_success || !vector.insert_called,
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                presentation.active_line,
                outcome
                    .presentation_requested
                    .then_some(ScriptTransferPresentationLine::InventoryMoved),
                "{}",
                vector.name
            );
            assert_eq!(
                request_flags.bits(),
                initial_request_flags
                    | if outcome.presentation_requested {
                        SECONDARY_PRESENTATION_REQUEST_BIT
                    } else {
                        u8::MIN
                    },
                "{}",
                vector.name
            );
        }
    }
}
