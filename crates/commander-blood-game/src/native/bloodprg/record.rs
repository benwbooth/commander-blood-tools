//! Typed relationship and topic fields used by BloodScript record operations.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::instruction::{ScriptDirectRecordOperation, ScriptRecordValue};
use commander_blood_formats::script::{ScriptObjectId, ScriptStateWord, ScriptWordId};

use super::{
    insert_aboard_object, remove_aboard_object, AboardObjectRoster, ScriptControl, ScriptRuntime,
    ScriptRuntimeError,
};

/// Owned typed values for relationship and topic fields reached by direct-record tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptRecordFields {
    values: BTreeMap<ScriptStateWord, ScriptRecordValue>,
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

/// Invalid typed state or control flow in one direct-record operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRecordError {
    /// The operation's typed target has not been initialized.
    MissingField {
        /// Missing field identity.
        field: ScriptStateWord,
    },
    /// An aboard transition targets state not owned by a profile object.
    MissingOwner {
        /// Ownerless field identity.
        field: ScriptStateWord,
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::ScriptCodeOffset;
    use commander_blood_formats::script::{
        decode_script_directory, decode_script_state, ScriptDirectory, ScriptState,
    };
    use serde::Deserialize;

    use super::*;

    const DIRECT_RECORD_VECTOR_COUNT: usize = 17;
    const QUERY_MODE_MASK: u8 = 1;
    const TOPIC_PUBLICATION_OPCODE: u8 = 0xBC;
    const BRANCH_TARGET: usize = 9_320;
    const TEST_FIELD_WORD_INDEX: usize = 1;

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

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn profile() -> (ScriptDirectory, ScriptState) {
        let directory =
            decode_script_directory(&std::fs::read(original_asset("SCRIPT1.DEB")).unwrap())
                .unwrap();
        let state = decode_script_state(
            &std::fs::read(original_asset("SCRIPT1.VAR")).unwrap(),
            &directory,
        )
        .unwrap();
        (directory, state)
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
        let (directory, state) = profile();
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
                assert!(record_runtime
                    .aboard_objects()
                    .slots()
                    .iter()
                    .all(Option::is_some));
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
}
