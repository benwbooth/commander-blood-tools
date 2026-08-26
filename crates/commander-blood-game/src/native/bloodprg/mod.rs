//! Native BLOODPRG game logic translated to flat, typed Rust data.

mod aboard;
mod alien_cycle;
mod actor_slots;
mod audio_bank;
mod audio_events;
mod audio_playback;
mod audio_stream;
mod bridge_input;
mod bridge_page;
mod bridge_panorama;
mod bridge_scene;
mod bridge_screen;
mod bridge_steering;
mod bridge_console;
mod actor_handler_black_hole;
mod actor_handler_camera;
mod actor_handler_hyperjump;
mod actor_handler_palette;
mod actor_handler_panel;
mod actor_handler_radio;
mod actor_position_state;
mod bridge_frame;
mod byte_move;
mod camera_approach;
mod camera_navigation;
mod choice_list;
mod clock;
mod descript;
mod descript_lookup;
mod diagnostics;
mod framebuffer_copy;
mod input_dispatch;
mod input_cancel;
mod input_selection;
mod location_panel;
mod location_panel_geometry;
mod menu_reveal;
mod name_area_effect;
mod navigation;
mod navigation_camera;
mod navigation_pick;
mod navigation_status;
mod navigation_wipe;
mod numbers;
mod palette_pipeline;
mod pbm_image;
mod presentation;
mod presentation_hover;
mod presentation_line;
mod presentation_mode;
mod presentation_word_choice;
mod pointer_buttons;
mod presentation_scan;
mod procedure;
mod record;
mod record_state;
mod resource_cache;
mod script;
mod script_action;
mod script_block;
mod script_control;
mod script_frame;
mod script_clock;
mod script_environment;
mod script_profile;
mod script_profile_request;
mod script_selector;
mod script_sequence_slots;
mod screen_presentation;
mod selected_mask;
mod sequence;
mod sequence_subtitles;
mod ship_depth;
mod ship_navigation;
mod ship_presentation;
mod ship_projection;
mod ship_target;
mod ship_view;
mod ship_hud;
mod ship_hud_coordinator;
mod sprite_geometry;
mod startup;
mod state;
mod subtitle_reveal;
mod text;
mod text_handler;
mod text_scan;
mod timer;
mod vm;

pub use aboard::{
    insert_aboard_object, rebuild_aboard_roster, remove_aboard_object, AboardObjectRoster,
    AboardRosterError, ABOARD_OBJECT_CAPACITY,
};
pub use alien_cycle::{
    AlienOverlayCycleHost, AlienOverlayCycleOutcome, AlienOverlayCycleState,
    AlienOverlayGraphicsTail, AlienOverlaySharedState, AlienOverlaySoundBank,
    AlienOverlayViewport, run_alien_overlay_cycle,
};
pub use actor_slots::{
    NAV_ACTOR_SLOT_COUNT, NavActorBusyState, NavActorHandler, NavActorMouseState,
    NavActorSeekState, NavActorSlot, NavActorSlotBackend, NavActorSlotFlags,
    NavActorSlotUpdateOutcome, deactivate_nav_actor_slots, update_nav_actor_slots,
};
pub use bridge_input::{
    STATUS_REGION_POLL_ATTEMPTS, PrimaryPointerSample, StatusRegionPollBackend,
    StatusRegionPollHit, latch_primary_pointer_hit, poll_status_region,
    primary_pointer_hits_region,
};
pub use bridge_page::{
    BridgePageBackend, BridgePageOutcome, BridgePageState, BridgePageTarget, render_bridge_page,
};
pub use bridge_panorama::{
    BridgePanoramaLoadTarget, BridgeStationOrbBoxes, load_bridge_panorama_frame,
};
pub use bridge_scene::{
    BridgeScene, BridgeSceneError, BridgeSceneFrame, BridgeSceneInput, INITIAL_BRIDGE_VIEW_FRAME,
};
pub use bridge_screen::{
    BRIDGE_CONSOLE_TINT_FIRST, BRIDGE_DARK_PALETTE_ADJUSTMENT, BridgePaletteAdjustment,
    BridgeScreenInitializationBackend, BridgeScreenInitializationOutcome,
    BridgeScreenInitializationPath, BridgeScreenInitializationState, initialize_bridge_screen,
};
pub use bridge_steering::{
    BRIDGE_ARC_UNIT_COUNT, BRIDGE_ARC_UNITS_PER_VIEW_FRAME, BRIDGE_CURSOR_RING_UNIT_COUNT,
    BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME, BRIDGE_LOGICAL_SCREEN_CENTER_X, BRIDGE_VIEW_FRAME_COUNT,
    BridgeSteeringInteraction, BridgeSteeringOutcome, BridgeSteeringState, BridgeTurnDirection,
    update_bridge_steering,
};
pub use actor_handler_black_hole::{
    BLACK_HOLE_IDLE_PRESENTATION_RESOURCE, BLACK_HOLE_TRANSITION_PRESENTATION_RESOURCE,
    BlackHoleActorPresentation, BlackHoleDeferredAction, BlackHoleNavigationTarget,
    BlackHolePresentationActorBackend, BlackHolePresentationActorContext,
    BlackHolePresentationActorOutcome, BlackHolePresentationActorState,
    BlackHolePresentationBlockers,
    update_black_hole_presentation_actor,
};
pub use actor_handler_camera::{
    CAMERA_TRANSITION_FRAME, CAMERA_VIEW_TRANSITION_STEPS, CameraActorPresentation,
    CameraPageFlipOutcome, CameraPresentationActorBackend, CameraPresentationActorOutcome,
    CameraPresentationActorState, CameraPresentationBlockers, CameraViewAnimation,
    update_camera_presentation_actor,
};
pub use actor_handler_hyperjump::{
    HYPERJUMP_IDLE_PRESENTATION_RESOURCE, HYPERJUMP_TRANSITION_PRESENTATION_RESOURCE,
    HyperjumpActorPresentation, HyperjumpDeferredAction, HyperjumpLocationPanelState,
    HyperjumpPresentationActorBackend, HyperjumpPresentationActorOutcome,
    HyperjumpPresentationActorState, update_hyperjump_presentation_actor,
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
pub use actor_position_state::{
    ActorPositionStateContext, ActorPositionStateError, ActorPositionStateOutcome,
    update_actor_position_states,
};
pub use bridge_frame::{
    BridgeActorPresentationState, BridgeFrameBackend, BridgeFrameOutcome, BridgeFrameState,
    BridgeSceneContext, BridgeSpriteRange, render_bridge_frame,
};
pub use audio_bank::{LoadedSoundBank, SoundBankUsage, load_sound_bank};
pub use audio_events::{
    AudioClipRequest, AudioEventContext, AudioEventError, AudioEventState, process_audio_events,
};
pub use audio_playback::{
    AudioDriverRequests, AudioMixOperation, AudioMixReport, AudioMixStatus, AudioPlaybackBanks,
    AudioPlaybackError, AudioPlaybackOutcome, AudioPlaybackState, AudioStreamBuffer,
    AudioStreamBufferStatus, DirectSoundPlayback, update_audio_playback,
};
pub use audio_stream::{
    AUDIO_STREAM_PAGE_BYTE_COUNT, AUDIO_STREAM_WAIT_PROMPT, AudioStreamError,
    AudioStreamLoadOutcome, AudioStreamPlaybackPosition, AudioStreamRefillOutcome,
    AudioStreamSource, AudioStreamStartOutcome, AudioStreamState, AudioStreamSubmission,
    AudioStreamSubmissionKind, CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT, load_audio_stream_source,
    refill_audio_stream, start_audio_stream,
};
pub use bridge_console::{
    BridgeChoiceBackend, BridgeChoicePanelPhase, BridgeConsoleActorState, BridgeConsoleChoice,
    BridgeConsoleContext, BridgeConsoleDispatchOutcome, BridgeConsoleGate,
    BridgeConsolePalettePlan, BridgeConsoleState, BridgeDeferredActionKind, BridgeDeferredRecord,
    BridgeDeferredState, BridgeRecordChoice, BridgeRecordChoiceContext, BridgeRecordChoiceOutcome,
    BridgeRecordChoiceState, ImmediateBridgeChoiceOutcome, MusicOptionLabel, OptionMenuChoice,
    OptionMenuOutcome, OptionMenuState, Rgb6, activate_horn_choice, activate_radio_choice,
    update_bridge_console_dispatch, update_contact_choice, update_navigation_target_choice,
    update_option_menu,
};
pub use byte_move::{ByteMoveError, move_bytes_in_place};
pub use camera_approach::{
    HYPERSPACE_SEQUENCE_COUNT, CameraApproachContext, CameraApproachHost,
    CameraApproachOutcome, CameraApproachPresentation, CameraApproachState,
    update_camera_approach,
};
pub use camera_navigation::{
    CameraNavigationLocation, CameraNavigationOutcome, CameraNavigationPaletteTransition,
    CameraNavigationPresentation, CameraNavigationRegionPoll, CameraNavigationShipMode,
    CameraNavigationSlot, CameraNavigationState, update_camera_navigation,
};
pub use choice_list::{
    CHOICE_LIST_ROW_PITCH, CHOICE_LIST_WIDTH_PADDING, ChoiceListBackend, ChoiceListConfig,
    ChoiceListFrame, ChoiceListPointer, ChoiceListPresentation, ChoiceListRect, ChoiceListRow,
    ChoiceListRowKind, ChoiceListState, update_choice_list,
};
pub use clock::{
    ScriptClockDate, decode_script_clock_date, decode_script_clock_hour,
};
pub use descript::{
    CachedDescriptBackground, DescriptBackgroundCache, DescriptBackgroundCacheOutcome,
    DescriptBackgroundSource, DescriptIdleClipSource, DescriptPresentationAssets,
    DescriptMusicSelectionOutcome, DescriptRecordBoundary, DescriptSoundBankLoader,
    append_descript_sequence_subtitle, append_descript_sequence_video, append_descript_talk_clip,
    cache_background_image, load_descript_idle_clip, load_descript_sound_bank,
    select_character_left_scene_video,
    select_character_right_scene_video, select_descript_character_sprite,
    select_descript_music, select_location_scene_video, select_object_scene_video,
    set_location_scene_top_row, stage_descript_caption, stop_before_character_record,
    stop_before_location_record, stop_before_object_record, stop_before_sequence_record,
};
pub use descript_lookup::{
    DescriptApplicationContext, DescriptApplicationError, DescriptApplicationResult,
    DescriptRecordApplication, lookup_and_apply_descript_record,
};
pub use diagnostics::{
    DiagnosticPanelLayout, DiagnosticRectangle, ErrorOverlay, ErrorOverlayLine,
    ErrorOverlayMode, UnknownErrorOverlayMode, build_error_overlay,
    calculate_diagnostic_panel_layout, write_diagnostic_console_text,
};
pub use framebuffer_copy::{
    FramebufferCopyError, FramebufferKind, LOGICAL_FRAMEBUFFER_HEIGHT,
    LOGICAL_FRAMEBUFFER_WIDTH, copy_work_surface_span,
};
pub use input_dispatch::{
    HostInputKey, IgnoredInputAction, InputAction, InputArrowKey, InputDispatchState,
    InputFunctionKey, dispatch_input_key, latch_input_text_byte, request_input_shutdown,
    toggle_input_pause, translate_input_key,
};
pub use input_cancel::{
    CANCELLATION_BLOCKED_LINE_FIRST, CANCELLATION_BLOCKED_LINE_LAST,
    CANCELLATION_DIALOGUE_READY_LINE, CANCELLATION_PALETTE_COLOR_COUNT,
    InputCancellationBackend, InputCancellationOutcome, InputCancellationState,
    PresentationResourceCursor, cancel_input_action,
};
pub use input_selection::{
    INPUT_SELECTION_VISIBLE_ROWS, InputDirectoryRowId, InputSelectionError,
    InputSelectionSource, InputSelectionState, SAVE_SLOT_NAME_LENGTH, SaveMenuState,
    SaveSlotName, accept_input_selection, move_input_selection_next,
    move_input_selection_previous,
};
pub use location_panel::{
    LocationInfoPanelContext, LocationInfoPanelHost, LocationInfoPanelOutcome,
    LocationInfoPanelState, LocationPanelArtwork, LocationPanelInterpolation,
    LocationPanelLocation, LocationPanelPhase, LocationPanelRect, LocationPanelRects,
    LocationPanelSource, LocationPanelSpriteRange, LocationPanelTextDraw,
    LocationPanelTransitionProgress, update_location_info_panel,
};
pub use location_panel_geometry::{
    LocationPanelGeometry, LocationPanelGeometryHost, LocationPanelGeometryState,
    LocationPanelLayout, update_location_panel_geometry,
};
pub use menu_reveal::{
    reveal_inline_menu_step, InlineMenuRevealError, InlineMenuRevealFrame,
    InlineMenuRevealGate, InlineMenuRevealOutcome, InlineMenuTextMetrics,
    InlineMenuWordPlacement,
};
pub use name_area_effect::{
    NameAreaEffectError, NameAreaEffectOutcome, NameAreaEffectState, update_name_area_effect,
};
pub use navigation::{
    navigation_actor_targets, navigation_candidates, navigation_chart_objects,
    navigation_distance, navigation_source_objects, object_links_to, objects_at_arche_position,
    presentable_navigation_objects, resolve_navigation_position, ScriptNavigationError,
};
pub use navigation_camera::{
    NavigationCameraContext, NavigationCameraError, NavigationCameraHost,
    NavigationCameraOutcome, NavigationCameraState, NavigationChartArche,
    NavigationChartCopySpan, NavigationChartEntityDraw, NavigationChartEntityState,
    NavigationChartHand, NavigationChartHandState, NavigationChartInputState,
    NavigationChartObject, NavigationChartObjectKind, NavigationChartWipeDirection,
    update_navigation_camera,
};
pub use navigation_pick::{
    NavigationChartMarkerEndpoint, NavigationChartPickObject, NavigationChartPickOutcome,
    NavigationChartPickState, pick_navigation_chart_object,
};
pub use navigation_status::{
    NavigationStatusContext, NavigationStatusHoverMode, NavigationStatusLabels,
    NavigationStatusLocation, NavigationStatusLocationKind, NavigationStatusOutcome,
    NavigationStatusRegion, NavigationStatusSource, NavigationStatusState, NavigationStatusText,
    update_navigation_status,
};
pub use navigation_wipe::{
    NAVIGATION_WIPE_CENTER_X, NAVIGATION_WIPE_CENTER_Y, NavigationWipeEndpointError,
    NavigationWipeSpan, build_navigation_wipe_spans,
};
pub use numbers::{
    append_decimal_i16, append_decimal_i32, packed_bcd_to_binary, parse_startup_audio_number,
    STARTUP_AUDIO_NUMBER_LENGTH,
};
pub use palette_pipeline::{
    PalettePipelineError, PaletteRemapTable, SCENE_PALETTE_CLEAR_COLOR_COUNT,
    TINT_PALETTE_BANK_SIZE, build_banked_tint_table, build_palette_blend_remap_table,
    clear_scene_palette_entries, interpolate_palette_range,
};
pub use pbm_image::{
    PBM_SCENE_PALETTE_COLOR_COUNT, PbmDecodeError, PbmDecodeOptions, PbmDecodeResult, PbmMarker,
    PbmPaletteUpdate, PbmTransparency, decode_pbm_image,
};
pub use presentation::{
    evaluate_text_conditions, ScriptWordHistory, TextConditionEffects, TextConditionError,
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
pub use presentation_mode::{
    PresentationBridgeMode, update_presentation_bridge_mode,
};
pub use presentation_word_choice::{
    WORD_CHOICE_TRANSITION_STEPS, PresentationWordChoice, PresentationWordChoiceBackend,
    PresentationWordChoiceContext, PresentationWordChoiceGate, PresentationWordChoiceOutcome,
    PresentationWordChoicePhase, PresentationWordChoiceState, update_presentation_word_choice,
};
pub use pointer_buttons::{
    PointerButton, PointerButtonEdges, PointerButtonState, PointerButtons,
    PointerLogicalRange, PointerSample, PointerSampleState, update_pointer_button_edges,
    update_pointer_sample,
};
pub use presentation_scan::{
    ScriptActionDispatch, ScriptActionDisposition, ScriptDeferredRecord,
    ScriptDeferredRecordKind, ScriptPresentationAction, ScriptPresentationEntity,
    ScriptPresentationHandoff, ScriptPresentationScanContext, ScriptPresentationScanError,
    ScriptPresentationScanHost, ScriptPresentationScanOutcome, ScriptPresentationScanState,
    deferred_navigation_record, scan_script_presentations,
};
pub use script_action::{
    ScriptActionContext, ScriptActionError, ScriptActionHost, ScriptActionPresentationLine,
    ScriptActionState, ScriptShipNavigationMode, ScriptTravelActionPhase,
    dispatch_script_action,
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
pub use resource_cache::{
    BLOODPRG_RESOURCE_CATALOG_FILE_OFFSET, ORIGINAL_RESOURCE_COUNT, OriginalResourceCache,
    OriginalResourceCatalog, PaletteResourceLoadOutcome, PaletteResourceStorage,
    PaletteResourceTarget, ResourceCacheError, ResourceId, ResourceLoadStatus,
};
pub use selected_mask::{
    PresentationChoiceMaskError, PresentationChoiceNumber, draw_presentation_choice_number,
};
pub use script::{
    ScriptControl, ScriptResumePhase, ScriptResumeState, ScriptRuntime, ScriptRuntimeError,
};
pub use script_block::{
    ScriptBlockEnd, ScriptBlockError, ScriptBlockFlow, ScriptBlockHandler, ScriptBlockOutcome,
    ScriptBlockStep, execute_script_block,
};
pub use script_control::{
    ScriptControlFlowContext, ScriptControlFlowError, ScriptControlFlowOutcome,
    execute_selector_control,
};
pub use script_frame::{
    ScriptFrameEnd, ScriptFrameError, ScriptFrameFlow, ScriptFrameHost, ScriptFrameOutcome,
    ScriptFrameStep, execute_script_frame,
};
pub use script_clock::ScriptClock;
pub use script_environment::ScriptEnvironmentActivity;
pub use script_profile::{
    BLOODPRG_SCRIPT_PROFILE_TABLE_FILE_OFFSET, ORIGINAL_SCRIPT_PROFILE_COUNT,
    SCRIPT_PROFILE_RESOURCE_COUNT, LoadedScriptProfile, OriginalScriptProfileCatalog,
    ScriptProfileBuiltins, ScriptProfileDataKind, ScriptProfileError, ScriptProfileId,
    ScriptProfileLoadOutcome, ScriptProfileManager, ScriptProfileResourceKind,
    ScriptProfileResources,
};
pub use script_profile_request::{
    PendingScriptProfileRequest, ScriptProfileRequestError, ScriptProfileRequestSlot,
};
pub use script_selector::{
    SCRIPT_CONCEPT_HISTORY_LENGTH, ScriptConceptHistory, ScriptSelectionError,
    ScriptSelectionOutcome, ScriptSelectorBranch, ScriptSelectorError, ScriptSelectorState,
    collect_selector_menu, commit_selected_concept, find_selector_body,
};
pub use script_sequence_slots::ScriptSequenceSlots;
pub use screen_presentation::{
    PresentationDescriptPlan, PresentationMusicChange, PresentationPanelPhase,
    PresentationPanelStep, PresentationRenderRegion, PresentationRenderTarget,
    PresentationResourcePlacement, PresentationSceneContext, PresentationSceneStatus,
    PresentationScreenBackend, PresentationScreenOutcome, PresentationScreenState,
    PresentationTextOrigin, PresentationTransitionFrame, update_presentation_screen,
};
pub use sequence::{
    load_sequence_request, offer_topic_if_presentation_active, PresentationResourceLine,
    SequencePresentationState, SequenceRequestContext,
};
pub use sequence_subtitles::{
    CenteredSequenceSubtitleLine, SequenceSubtitleOutcome, SequenceSubtitlePlayback,
    SequenceSubtitleRenderer, present_sequence_subtitle,
};
pub use subtitle_reveal::{
    SubtitleFrameDraw, SubtitleFramePrimitive, SubtitleFramePrimitiveKind, SubtitleRevealError,
    SubtitleRevealGate, SubtitleRevealLine, SubtitleRevealOutcome, SubtitleRevealPhase,
    SubtitleRevealRenderer, SubtitleRevealState, update_subtitle_reveal,
};
pub use ship_projection::{
    FULL_SHIP_PROJECTION_CLIP, SHIP_OBJECT_ANCHOR_COUNT, SHIP_POINT_CLOUD_COUNT,
    SHIP_TRIGONOMETRY_SAMPLE_COUNT, ShipCameraPosition, ShipObjectAnchor,
    ShipObjectSpriteProjection, ShipPlottedPoint, ShipPointCloudProjection, ShipPointRecord,
    ShipProjectedPoint, ShipProjectionAngleRole, ShipProjectionAngles, ShipProjectionClip,
    ShipProjectionError, ShipProjectionMatrix, ShipProjectionResources, ShipTrigonometrySample,
    build_ship_projection_matrix, plot_ship_point, project_ship_object_sprites,
    project_ship_point_cloud, randomize_ship_point_cloud,
};
pub use ship_target::{
    ShipTargetListPass, ShipTargetListSelection, ShipTargetListSource, ShipTargetSelectionError,
    ShipTargetSelectionHost, ShipTargetSelectionOutcome, ShipTargetSelectionState,
    select_ship_target,
};
pub use ship_view::{
    select_ship_view_artwork, ShipViewArtworkError, ShipViewArtworkSelection,
    ShipViewEntityId, ShipViewEntityPlacement, ShipViewResourceId, ShipViewResourceRequest,
};
pub use ship_hud::{
    SHIP_CAMERA_RESET, SHIP_HUD_PALETTE_COLOR_COUNT, SHIP_HUD_PALETTE_FIRST, IndexedGamePalette,
    ShipHudBackend, ShipHudPaletteSnapshot, ShipHudState,
    snapshot_ship_hud_palette_and_reset_camera,
};
pub use ship_hud_coordinator::{
    ShipHudCoordinatorError, ShipHudCoordinatorHost, ShipHudCoordinatorOutcome,
    ShipHudCoordinatorState, ShipHudInitializationContext, ShipHudPaletteTransition,
    ShipHudTargetListState, update_ship_hud,
};
pub use ship_depth::{
    ShipDepthBandLayout, ShipDepthTransition, ShipDepthTransitionOutcome,
    advance_ship_depth, prepare_ship_depth_band,
};
pub use ship_navigation::{
    ShipNavigationAccessCounter, ShipNavigationCandidate, ShipNavigationContext,
    ShipNavigationHost, ShipNavigationOutcome, ShipNavigationRelation, ShipNavigationState,
    update_ship_navigation,
};
pub use ship_presentation::{
    ShipPresentationHost, ShipPresentationOutcome, ShipPresentationState,
    update_ship_presentation,
};
pub use sprite_geometry::{
    BRIDGE_SPRITE_ENTITY_COUNT, BridgeSpriteEntity, BridgeSpriteEntityError, BridgeSpriteExtent,
    BridgeSpriteFlags, BridgeSpriteGeometryUpdate, BridgeSpritePosition,
    update_bridge_sprite_extent, update_bridge_sprite_position,
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
pub use timer::{
    GameTimerContext, GameTimerState, GameTimerTickOutcome, GameTimerTickStatus,
    SpeakerGateAction, SpeakerPulseState, advance_game_timer_tick,
};
pub use vm::{
    active_objects_in_play, count_positive_operands, object_before_threshold, object_has_flag,
    resolve_dictionary_object, script_field_offset, ScriptFieldSelector, ScriptObjectFlag,
    set_object_flag,
};
