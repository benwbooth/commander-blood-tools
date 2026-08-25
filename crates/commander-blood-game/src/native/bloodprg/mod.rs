//! Native BLOODPRG game logic translated to flat, typed Rust data.

mod aboard;
mod descript;
mod menu_reveal;
mod navigation;
mod numbers;
mod presentation;
mod procedure;
mod record;
mod record_state;
mod script;
mod sequence;
mod ship_view;
mod startup;
mod state;
mod text;
mod text_handler;
mod text_scan;
mod vm;

pub use aboard::{
    insert_aboard_object, remove_aboard_object, AboardObjectRoster, ABOARD_OBJECT_CAPACITY,
};
pub use descript::{
    CachedDescriptBackground, DescriptBackgroundCache, DescriptBackgroundCacheOutcome,
    DescriptBackgroundSource, DescriptIdleClipSource, DescriptPresentationAssets,
    DescriptRecordBoundary, DescriptSoundBankLoader, append_descript_talk_clip,
    cache_background_image, load_descript_idle_clip, load_descript_sound_bank,
    select_character_left_scene_video,
    select_character_right_scene_video, select_location_scene_video, select_object_scene_video,
    set_location_scene_top_row, stage_descript_caption, stop_before_character_record,
    stop_before_location_record, stop_before_object_record, stop_before_sequence_record,
};
pub use menu_reveal::{
    reveal_inline_menu_step, InlineMenuRevealError, InlineMenuRevealFrame,
    InlineMenuRevealGate, InlineMenuRevealOutcome, InlineMenuTextMetrics,
    InlineMenuWordPlacement,
};
pub use navigation::{
    navigation_actor_targets, navigation_candidates, navigation_chart_objects,
    navigation_distance, navigation_source_objects, object_links_to, objects_at_arche_position,
    presentable_navigation_objects, resolve_navigation_position, ScriptNavigationError,
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
    apply_direct_record_operation, apply_record_pair_operation, apply_transfer, ScriptRecordError,
    ScriptRecordFields, ScriptRecordPairReference, ScriptRecordRuntime, ScriptTransferContext,
    ScriptTransferOutcome, ScriptTransferPresentationLine, ScriptTransferPresentationState,
    ScriptTransferRecord, ScriptTransferRecords,
};
pub use record_state::{
    apply_aboard_record_operation, apply_active_object_record_operation,
    apply_actor_record_operation, apply_presentation_queue_operation,
    apply_opaque_marker_record_operation, apply_record_clear_operation,
    apply_record_state_operation, apply_travel_record_operation,
    apply_world_state_record_operation, ScriptAboardPresentationLine,
    ScriptAboardPresentationState, ScriptAboardRecordContext, ScriptAboardRecordOutcome,
    ScriptActionRecord, ScriptActionRecords, ScriptRecordClearOutcome,
    ScriptRecordClearPresentationState, ScriptRecordStateError,
    ScriptRecordStateNavigationContext, ScriptRecordStateOutcome,
};
pub use script::{ScriptControl, ScriptResumeState, ScriptRuntime, ScriptRuntimeError};
pub use sequence::{
    load_sequence_request, offer_topic_if_presentation_active, PresentationResourceLine,
    SequencePresentationState, SequenceRequestContext,
};
pub use ship_view::{
    select_ship_view_artwork, ShipViewArtworkError, ShipViewArtworkSelection,
    ShipViewEntityId, ShipViewEntityPlacement, ShipViewResourceId, ShipViewResourceRequest,
};
pub use startup::{
    apply_startup_option, tokenize_startup_command, StartupAudioConfiguration, StartupAudioDriver,
    StartupConfiguration,
};
pub use state::{
    apply_bit_flag_operation, apply_shared_bit_operation, apply_shared_state_operation,
    ScriptStateOperationError,
};
pub use text::{bounded_nul_byte_len, nul_terminated_byte_len, nul_terminated_bytes_equal};
pub use text_handler::{
    handle_text_instruction, PresentationRequestFlags, TextConditionInputs, TextHandlerError,
    TextHandlerGate, TextHandlerOutcome, TextInstructionState, TextLineKind, TextLineState,
    TextPresentationState,
};
pub use text_scan::{
    activate_object_text, BoundTextInstruction, ScriptTextActivationError,
    ScriptTextActivationRegistry,
};
pub use vm::{
    active_objects_in_play, count_positive_operands, object_before_threshold, object_has_flag,
    resolve_dictionary_object, script_field_offset, ScriptFieldSelector, ScriptObjectFlag,
};
