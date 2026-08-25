//! Native BLOODPRG game logic translated to flat, typed Rust data.

mod aboard;
mod actor_handler_palette;
mod actor_handler_panel;
mod actor_handler_radio;
mod actor_slots;
mod bridge_frame;
mod camera_navigation;
mod descript;
mod descript_lookup;
mod menu_reveal;
mod navigation;
mod numbers;
mod presentation;
mod presentation_hover;
mod presentation_line;
mod procedure;
mod record;
mod record_state;
mod screen_presentation;
mod script;
mod selected_mask;
mod sequence;
mod sequence_subtitles;
mod ship_view;
mod startup;
mod state;
mod text;
mod text_handler;
mod text_scan;
mod vm;

pub use aboard::{
    ABOARD_OBJECT_CAPACITY, AboardObjectRoster, insert_aboard_object, remove_aboard_object,
};
pub use actor_handler_palette::{
    SHIP_ACTOR_PALETTE_BYTES, ShipPaletteActorBackend, ShipPaletteActorOutcome,
    ShipPaletteActorPresentation, ShipPaletteActorState, update_ship_palette_actor,
};
pub use actor_handler_panel::{
    PanelCloseActorBackend, PanelCloseActorOutcome, PanelCloseActorPresentation,
    PanelCloseActorState, update_panel_close_actor,
};
pub use actor_handler_radio::{
    RadioActorBackend, RadioActorDeferredAction, RadioActorOutcome, RadioActorPresentation,
    RadioActorState, update_radio_actor,
};
pub use actor_slots::{
    NAV_ACTOR_SLOT_COUNT, NavActorBusyState, NavActorHandler, NavActorMouseState,
    NavActorSeekState, NavActorSlot, NavActorSlotBackend, NavActorSlotFlags,
    NavActorSlotUpdateOutcome, update_nav_actor_slots,
};
pub use bridge_frame::{
    BridgeActorPresentationState, BridgeFrameBackend, BridgeFrameOutcome, BridgeFrameState,
    BridgeSceneContext, BridgeSpriteRange, render_bridge_frame,
};
pub use camera_navigation::{
    CameraNavigationLocation, CameraNavigationOutcome, CameraNavigationPaletteTransition,
    CameraNavigationPresentation, CameraNavigationRegionPoll, CameraNavigationShipMode,
    CameraNavigationSlot, CameraNavigationState, update_camera_navigation,
};
pub use descript::{
    CachedDescriptBackground, DescriptBackgroundCache, DescriptBackgroundCacheOutcome,
    DescriptBackgroundSource, DescriptIdleClipSource, DescriptMusicSelectionOutcome,
    DescriptPresentationAssets, DescriptRecordBoundary, DescriptSoundBankLoader,
    append_descript_sequence_subtitle, append_descript_sequence_video, append_descript_talk_clip,
    cache_background_image, load_descript_idle_clip, load_descript_sound_bank,
    select_character_left_scene_video, select_character_right_scene_video,
    select_descript_character_sprite, select_descript_music, select_location_scene_video,
    select_object_scene_video, set_location_scene_top_row, stage_descript_caption,
    stop_before_character_record, stop_before_location_record, stop_before_object_record,
    stop_before_sequence_record,
};
pub use descript_lookup::{
    DescriptApplicationContext, DescriptApplicationError, DescriptApplicationResult,
    DescriptRecordApplication, lookup_and_apply_descript_record,
};
pub use menu_reveal::{
    InlineMenuRevealError, InlineMenuRevealFrame, InlineMenuRevealGate, InlineMenuRevealOutcome,
    InlineMenuTextMetrics, InlineMenuWordPlacement, reveal_inline_menu_step,
};
pub use navigation::{
    ScriptNavigationError, navigation_actor_targets, navigation_candidates,
    navigation_chart_objects, navigation_distance, navigation_source_objects, object_links_to,
    objects_at_arche_position, presentable_navigation_objects, resolve_navigation_position,
};
pub use numbers::{
    STARTUP_AUDIO_NUMBER_LENGTH, append_decimal_i16, append_decimal_i32, packed_bcd_to_binary,
    parse_startup_audio_number,
};
pub use presentation::{
    ScriptWordHistory, TextConditionEffects, TextConditionError, evaluate_text_conditions,
};
pub use presentation_hover::{
    PresentationHitAreas, PresentationHitRectangle, PresentationHitSelection,
    PresentationHoverOutcome, PresentationHoverState, update_presentation_hover,
};
pub use presentation_line::{
    PresentationLine, PresentationLineBackend, PresentationLineFlags, PresentationLineOutcome,
    PresentationLinePlayback, PresentationLineStepper, PresentationResourceId,
    update_presentation_line,
};
pub use procedure::{
    ScriptProcedureStateError, ScriptProcedureStates, apply_procedure_activation,
    evaluate_procedure_gate,
};
pub use record::{
    ScriptRecordError, ScriptRecordFields, ScriptRecordPairReference, ScriptRecordRuntime,
    ScriptTransferContext, ScriptTransferOutcome, ScriptTransferPresentationLine,
    ScriptTransferPresentationState, ScriptTransferRecord, ScriptTransferRecords,
    apply_direct_record_operation, apply_record_pair_operation, apply_transfer,
};
pub use record_state::{
    ScriptAboardPresentationLine, ScriptAboardPresentationState, ScriptAboardRecordContext,
    ScriptAboardRecordOutcome, ScriptActionRecord, ScriptActionRecords, ScriptRecordClearOutcome,
    ScriptRecordClearPresentationState, ScriptRecordStateError, ScriptRecordStateNavigationContext,
    ScriptRecordStateOutcome, apply_aboard_record_operation, apply_active_object_record_operation,
    apply_actor_record_operation, apply_opaque_marker_record_operation,
    apply_presentation_queue_operation, apply_record_clear_operation, apply_record_state_operation,
    apply_travel_record_operation, apply_world_state_record_operation,
};
pub use screen_presentation::{
    PresentationDescriptPlan, PresentationMusicChange, PresentationPanelPhase,
    PresentationPanelStep, PresentationRenderRegion, PresentationRenderTarget,
    PresentationResourcePlacement, PresentationSceneContext, PresentationSceneStatus,
    PresentationScreenBackend, PresentationScreenOutcome, PresentationScreenState,
    PresentationTextOrigin, PresentationTransitionFrame, update_presentation_screen,
};
pub use script::{ScriptControl, ScriptResumeState, ScriptRuntime, ScriptRuntimeError};
pub use selected_mask::{
    PresentationChoiceMaskError, PresentationChoiceNumber, draw_presentation_choice_number,
};
pub use sequence::{
    PresentationResourceLine, SequencePresentationState, SequenceRequestContext,
    load_sequence_request, offer_topic_if_presentation_active,
};
pub use sequence_subtitles::{
    CenteredSequenceSubtitleLine, SequenceSubtitleOutcome, SequenceSubtitlePlayback,
    SequenceSubtitleRenderer, present_sequence_subtitle,
};
pub use ship_view::{
    ShipViewArtworkError, ShipViewArtworkSelection, ShipViewEntityId, ShipViewEntityPlacement,
    ShipViewResourceId, ShipViewResourceRequest, select_ship_view_artwork,
};
pub use startup::{
    StartupAudioConfiguration, StartupAudioDriver, StartupConfiguration, apply_startup_option,
    tokenize_startup_command,
};
pub use state::{
    ScriptStateOperationError, apply_bit_flag_operation, apply_shared_bit_operation,
    apply_shared_state_operation,
};
pub use text::{bounded_nul_byte_len, nul_terminated_byte_len, nul_terminated_bytes_equal};
pub use text_handler::{
    PresentationRequestFlags, TextConditionInputs, TextHandlerError, TextHandlerGate,
    TextHandlerOutcome, TextInstructionState, TextLineKind, TextLineState, TextPresentationState,
    handle_text_instruction,
};
pub use text_scan::{
    BoundTextInstruction, ScriptTextActivationError, ScriptTextActivationRegistry,
    activate_object_text,
};
pub use vm::{
    ScriptFieldSelector, ScriptObjectFlag, active_objects_in_play, count_positive_operands,
    object_before_threshold, object_has_flag, resolve_dictionary_object, script_field_offset,
};
