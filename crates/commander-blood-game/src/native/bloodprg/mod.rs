//! Native BLOODPRG game logic translated to flat, typed Rust data.

mod aboard;
mod numbers;
mod presentation;
mod procedure;
mod record;
mod script;
mod sequence;
mod startup;
mod state;
mod text;
mod text_handler;
mod vm;

pub use aboard::{
    insert_aboard_object, remove_aboard_object, AboardObjectRoster, ABOARD_OBJECT_CAPACITY,
};
pub use numbers::{
    append_decimal_i16, append_decimal_i32, packed_bcd_to_binary, parse_startup_audio_number,
    STARTUP_AUDIO_NUMBER_LENGTH,
};
pub use presentation::{
    evaluate_text_conditions, ScriptWordHistory, TextConditionEffects, TextConditionError,
};
pub use procedure::{
    apply_procedure_activation, evaluate_procedure_gate, ScriptProcedureStateError,
    ScriptProcedureStates,
};
pub use record::{
    apply_direct_record_operation, ScriptRecordError, ScriptRecordFields, ScriptRecordRuntime,
};
pub use script::{ScriptControl, ScriptResumeState, ScriptRuntime, ScriptRuntimeError};
pub use sequence::{
    load_sequence_request, offer_topic_if_presentation_active, PresentationResourceLine,
    SequencePresentationState, SequenceRequestContext,
};
pub use startup::{
    apply_startup_option, tokenize_startup_command, StartupAudioConfiguration, StartupAudioDriver,
    StartupConfiguration,
};
pub use state::{
    apply_shared_bit_operation, apply_shared_state_operation, ScriptStateOperationError,
};
pub use text::{bounded_nul_byte_len, nul_terminated_byte_len, nul_terminated_bytes_equal};
pub use text_handler::{
    handle_text_instruction, PresentationRequestFlags, TextConditionInputs, TextHandlerError,
    TextHandlerGate, TextHandlerOutcome, TextInstructionState, TextLineKind, TextLineState,
    TextPresentationState,
};
pub use vm::{
    count_positive_operands, object_before_threshold, resolve_dictionary_object,
    script_field_offset, ScriptFieldSelector,
};
