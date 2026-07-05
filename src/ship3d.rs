use crate::vm;

pub const SHIP_3D_MAX_DEPTH_OFFSET: u16 = 0x41;
pub const SHIP_3D_TRANSITION_OPEN_STEP: u8 = 0x04;
pub const SHIP_3D_TRANSITION_CLOSE_STEP: u8 = 0x08;
pub const SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD: u16 = 0x78;
pub const SHIP_3D_PLANE_ROW_BYTES: usize = 0x50;
pub const SHIP_3D_PLANE_PAGE_BYTES: usize = 0x1f40;
pub const SHIP_3D_PLANE_BASE_ROWS: usize = 0x23;
pub const SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET: usize = 0xc000;
pub const SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET: usize = 0xdf40;
pub const SHIP_3D_PLANE_DEST_BYTES: usize = SHIP_3D_PLANE_PAGE_BYTES * 2;
pub const SHIP_3D_SCROLL_MODE_HOLD: u16 = 0x000a;
pub const SHIP_3D_TARGET_EXIT_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_TARGET_RECORD_HEADER_BYTES: u16 = 0x0004;
pub const SHIP_3D_TARGET_OPEN_STEP: u8 = 0x06;
pub const SHIP_3D_INTERPOLATION_WORDS: usize = 4;
pub const SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH: u16 = 0x64;
pub const SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH: u16 = 0x37;
pub const SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING: u16 = 0x14;
pub const SHIP_3D_TARGET_LAYOUT_ROW_STEP: u16 = 0x0b;
pub const SHIP_3D_TARGET_LAYOUT_EXTRA_HEIGHT: u16 = 0x0a;
pub const SHIP_3D_TARGET_LAYOUT_HEIGHT_PADDING: u16 = 0x08;
pub const SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT: u16 = 0xc8;
pub const SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN: u16 = 0xffff;
pub const SHIP_3D_TARGET_HIT_TEST_TOP_INSET: u16 = 0x04;
pub const SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET: u16 = 0x08;
pub const SHIP_3D_TARGET_HOVER_PRESENTATION_MODE: u16 = 0x0006;
pub const SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE: u16 = 0x0007;
pub const SHIP_3D_TARGET_IDLE_PRESENTATION_MODE: u16 = 0x0001;
pub const SHIP_3D_TARGET_DRAW_X_INSET: u16 = 0x0a;
pub const SHIP_3D_TARGET_DEFAULT_TEXT_COLOR: u8 = 0xe8;
pub const SHIP_3D_TARGET_HOVER_TEXT_COLOR: u8 = 0xef;
pub const SHIP_3D_TARGET_ACTIVE_TEXT_COLOR: u8 = 0xfe;
pub const SHIP_3D_TARGET_EXTRA_LABEL_OFFSET: u16 = 0x0174;
pub const SHIP_3D_TARGET_ALIAS_LABEL_OFFSET: u16 = 0x273b;
pub const SHIP_3D_NAV_CHOICE_MIN_GATE: u16 = 0x28;
pub const SHIP_3D_NAV_CHOICE_MAX_GATE: u16 = 0x3c;
pub const SHIP_3D_NAV_CHOICE_AXIS_BIAS: u16 = 0x2d;
pub const SHIP_3D_NAV_CHOICE_RIGHT_BASE: u16 = 0x011f;
pub const SHIP_3D_NAV_CHOICE_X_WIDTH: u16 = 0x006e;
pub const SHIP_3D_NAV_CHOICE_Y_BASE: u16 = 0x0048;
pub const SHIP_3D_NAV_CHOICE_ROW_HEIGHT_BASE: u8 = 0x12;
pub const SHIP_3D_NAV_CHOICE_COUNT: u8 = 5;
pub const SHIP_3D_NAV_CHOICE_PALETTE_FIRST: u8 = 0x7b;
pub const SHIP_3D_NAV_CHOICE_PRESENTATION_MODE: u16 = 0x0005;
pub const SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS: u8 = 0x0c;
pub const SHIP_3D_NAV_CHOICE_DISPATCH_BLOCK_FLAG: u8 = 0x08;
pub const SHIP_3D_NAV_CHOICE_HOLD_TICKS: u16 = 0x005a;
pub const SHIP_3D_NAV_CHOICE_HANDLER_PHASE: u8 = 0x01;
pub const SHIP_3D_NAV_CHOICE_TARGET_Y_BASE: u16 = 0x0050;
pub const SHIP_3D_NAV_CHOICE_TARGET_Y_STEP: u16 = 0x0012;
pub const SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X: u16 = 0x0064;
pub const SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION: u8 = 0x0a;
pub const SHIP_3D_NAV_CHOICE_SELECT_SOUND: u16 = 0x0004;
pub const SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE: u16 = 0x00c3;
pub const SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG: u8 = 0x04;
pub const SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING: u8 = 0x02;
pub const SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET: u16 = 0x0d16;
pub const SHIP_3D_NAV_CHOICE_HANDLER4_TARGET_LIST_OFFSET: u16 = 0x2567;
pub const SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_OFF_TARGET_LIST_OFFSET: u16 = 0x2578;
pub const SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_ON_TARGET_LIST_OFFSET: u16 = 0x2581;
pub const SHIP_3D_NAV_CHOICE_TABLO2_VOC_PATH_OFFSET: u16 = 0x0d3d;
pub const SHIP_3D_NAV_CHOICE_SOUND_GATE_SUPPRESS_TARGETS: u8 = 0x02;
pub const SHIP_3D_NAVIGATION_INTERPOLATION_DURATION: u8 = 0x06;
pub const SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE: u16 = 0x00c4;
pub const SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE: u16 = 0x0002;
pub const SHIP_3D_NAVIGATION_RECORD_ACTIVE_FLAG: u8 = 0x01;
pub const SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG: u8 = 0x02;
pub const SHIP_3D_NAVIGATION_REDIRECT_COUNTER_FLAG: u16 = 0x0080;
pub const SHIP_3D_NAVIGATION_TARGET_LIST_FLAG: u8 = 0x04;
pub const SHIP_3D_NAVIGATION_LAYOUT_TARGET_LIST_OFFSET: u16 = 0x253b;
pub const SHIP_3D_NAVIGATION_SCENE_BAND_TOP: u16 = 0x0023;
pub const SHIP_3D_NAVIGATION_RENDER_CLIP_BOTTOM: u16 = 0x00a5;
pub const SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM: u16 = 0x00c8;
pub const SHIP_3D_NAVIGATION_TRIGGER_CLOSE_STEP: u8 = 0x02;
pub const SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG: u16 = 0x0008;
pub const SHIP_3D_PROCEDURAL_TARGET_LIST_FLAG: u16 = 0x0004;
pub const SHIP_3D_PROCEDURAL_HALF_TURN: u16 = 0x00b4;
pub const SHIP_3D_PROCEDURAL_FULL_TURN: u16 = 0x0168;
pub const SHIP_3D_PROCEDURAL_MOUSE_RING: u16 = 0x05a0;
pub const SHIP_3D_PROCEDURAL_MOUSE_CENTER_X: u16 = 0x05a0;
pub const SHIP_3D_PROCEDURAL_MOUSE_ALIGN_MASK: u16 = 0xfff8;
pub const SHIP_3D_PROCEDURAL_CLOSE_ANGLE_THRESHOLD: u16 = 0x001f;
pub const SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD: u16 = 0x0028;
pub const SHIP_3D_PROCEDURAL_TARGET_LIST_STEP: u16 = 0x0028;
pub const SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP: u16 = 0x001e;
pub const SHIP_3D_PROCEDURAL_ROTATION_OFFSET_BIAS: u16 = 0x00a0;
pub const SHIP_3D_MATRIX_ANGLE_TABLE_OFFSET: u16 = 0x4f45;
pub const SHIP_3D_MATRIX_ANGLE_A_OFFSET: u16 = 0x2f71;
pub const SHIP_3D_MATRIX_PROJECTION_ANGLE_OFFSET: u16 = 0x2f6d;
pub const SHIP_3D_MATRIX_ANGLE_C_OFFSET: u16 = 0x2f6f;
pub const SHIP_3D_MATRIX_TEMP_OFFSET: u16 = 0x2f7d;
pub const SHIP_3D_PROJECTION_MATRIX_OFFSET: u16 = 0x2f95;
pub const SHIP_3D_MATRIX_FIXED_SHIFT: u8 = 0x0f;
pub const SHIP_3D_PROJECTION_CAMERA_X_OFFSET: u16 = 0x2f65;
pub const SHIP_3D_PROJECTION_CAMERA_Y_OFFSET: u16 = 0x2f67;
pub const SHIP_3D_PROJECTION_CAMERA_Z_OFFSET: u16 = 0x2f69;
pub const SHIP_3D_POINT_CLOUD_COUNT: usize = 0x03e8;
pub const SHIP_3D_POINT_BUFFER_OFFSET: u16 = 0x2fc1;
pub const SHIP_3D_PROJECTION_WORK_VECTOR_OFFSET: u16 = 0x4f01;
pub const SHIP_3D_PROJECTED_X_OFFSET: u16 = 0x2fb9;
pub const SHIP_3D_PROJECTED_Y_OFFSET: u16 = 0x2fbb;
pub const SHIP_3D_PROJECTED_DEPTH_OFFSET: u16 = 0x2fbd;
pub const SHIP_3D_PROJECTION_VIEWPORT_LEFT_OFFSET: u16 = 0x5235;
pub const SHIP_3D_PROJECTION_VIEWPORT_RIGHT_OFFSET: u16 = 0x5237;
pub const SHIP_3D_PROJECTION_VIEWPORT_TOP_OFFSET: u16 = 0x5239;
pub const SHIP_3D_PROJECTION_VIEWPORT_BOTTOM_OFFSET: u16 = 0x523b;
pub const SHIP_3D_PROJECTION_SCREEN_CENTER_X: u16 = 0x00a0;
pub const SHIP_3D_PROJECTION_SCREEN_CENTER_Y: u16 = 0x0064;
pub const SHIP_3D_PROJECTION_SCREEN_WIDTH: usize = 0x0140;
pub const SHIP_3D_PROJECTION_AXIS_SHIFT: u8 = 0x07;
pub const SHIP_3D_PROJECTION_SHADE_SHIFT: u8 = 0x0c;
pub const SHIP_3D_PROJECTION_SHADE_BASE: u8 = 0xef;
pub const SHIP_3D_OBJECT_ANCHOR_OFFSET: u16 = 0x4f09;
pub const SHIP_3D_OBJECT_ANCHOR_COUNT: usize = 0x0b;
pub const SHIP_3D_OBJECT_ANCHOR_STRIDE: u16 = 0x0006;
pub const SHIP_3D_OBJECT_DESCRIPTOR_BASE_OFFSET: u16 = 0x6212;
pub const SHIP_3D_OBJECT_DESCRIPTOR_STRIDE: u16 = 0x0020;
pub const SHIP_3D_OBJECT_DESCRIPTOR_INDEX_BIAS: u16 = 0x0015;
pub const SHIP_3D_OBJECT_VISIBLE_FLAG: u16 = 0x0080;
pub const SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG: u16 = 0x0001;
pub const SHIP_3D_SPRITE_SLOT_ACTIVE_MASK: u16 = 0x0081;
pub const SHIP_3D_SPRITE_SLOT_DIRTY_FLAG: u16 = 0x0002;
pub const SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG: u16 = 0x0010;
pub const SHIP_3D_OBJECT_DEPTH_WRAP_BIAS: i32 = 0x0001_0000;
pub const SHIP_3D_OBJECT_SCALE_NUMERATOR: u32 = 0x0010_0000;
pub const SHIP_3D_OBJECT_SCALE_SHIFT: u8 = 0x0a;
pub const SHIP_3D_OBJECT_PROJECTED_SCALE_OFFSET: u16 = 0x2fbf;
pub const SHIP_3D_GLOBAL_CLIP_SNAPSHOT_FLAG_OFFSET: u16 = 0x5249;
pub const SHIP_3D_DIRTY_RECT_LIST_OFFSET: u16 = 0x6612;
pub const SHIP_3D_DIRTY_RECT_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_TEMP_SND_CALLBACK_TABLE_OFFSET: u16 = 0x0acc;
pub const SHIP_3D_TEMP_SND_CALLBACK_OFFSETS: [u16; 3] = [0x0087, 0x0090, 0x009c];
pub const SHIP_3D_TEMP_SND_PATH_OFFSET: u16 = 0x0d23;
pub const SHIP_3D_TB_SND_PATH_OFFSET: u16 = 0x0cfc;
pub const SHIP_3D_TEMP_SND_PHASE_COUNT: u8 = 3;
pub const SHIP_3D_TEMP_SND_SCENE_SELECTOR_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR: [u16; 8] = [
    0x0000, 0x0001, 0x0004, 0x0000, 0x0140, 0x00c8, 0x0000, 0x0000,
];
pub const SHIP_3D_FINAL_RESET_HUD_FLAGS: u16 = 0x0009;
pub const SHIP_3D_FINAL_RESET_NAV_TIMER: u16 = 0x0032;
pub const SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_FINAL_RESET_ACTIVE_RECORD_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_FINAL_RESET_DIRTY_MARKER: u8 = 0xff;
pub const SHIP_3D_FINAL_RESET_SCROLL_MODE: u16 = SHIP_3D_SCROLL_MODE_HOLD;
pub const SHIP_3D_FINAL_RESET_STATUS_FLAG_MASK: u8 = 0xfc;
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8: u16 = 0x0008;
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10: u16 = 0x0010;
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40: u16 = 0x0040;
pub const SHIP_3D_OBJECT_KIND_POSITION_KIND100: u16 = 0x0100;
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200: u16 = 0x0200;
pub const SHIP_3D_FIELD_SELECTOR_POSITION: u8 = 0x0b;
pub const SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH: u8 = 0x09;
pub const SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH: u8 = 0x0a;
pub const SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD: u8 = 0x0c;
pub const SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD: u8 = 0x0e;
pub const SHIP_3D_FIELD_SELECTOR_PARENT_LINK: u8 = 0x11;
pub const SHIP_3D_SOURCE_BITSET_SELECTOR: u8 = 0x05;
pub const SHIP_3D_SOURCE_BITSET_KIND: u16 = 0x0002;
pub const SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG: u16 = 0x0001;
pub const SHIP_3D_C1_SOURCE_KIND_BITSET: u16 = 0x0002;
pub const SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG: u8 = 0x02;
pub const SHIP_3D_C1_KIND10_RECORD_KIND: u16 = 0x0010;
pub const SHIP_3D_C1_DESTINATION_SELECTOR: u8 = 0x13;
pub const SHIP_3D_C1_RECORD_STATE_OPCODE: u16 = vm::OP_RECORD_STATE_MIN as u16;
pub const SHIP_3D_C1_RECORD_STATE_AUX_WORD: u16 = 0x0002;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTransitionState {
    pub hold_ticks: u16,
    pub transition_armed: bool,
    pub opening: bool,
    pub closing: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dDepthState {
    pub depth_offset: u16,
    pub opening: bool,
    pub closing: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dPlaneBandCopy {
    pub row_count: usize,
    pub byte_count: usize,
    pub first_source_start: usize,
    pub first_dest_start: usize,
    pub second_source_start: usize,
    pub second_dest_start: usize,
    pub new_scroll_value: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTargetSelectorState {
    pub current_target: u16,
    pub target_select_phase: u8,
    pub target_fallback: bool,
    pub target_animation_tick: u8,
    pub opening: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetSelection {
    pub ax: u16,
    pub used_fallback_table: bool,
    pub ran_layout_prepass: bool,
    pub phase_gate_blocked: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dInterpolationGate {
    pub duration_ticks: u8,
    pub current_tick: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ship3dInterpolationStep {
    Active([u16; SHIP_3D_INTERPOLATION_WORDS]),
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetListLayout {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub max_label_width: u16,
    pub label_count: usize,
    pub has_extra_entry: bool,
    pub selector_mode_return_ax: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTargetHitState {
    pub hover_row: u8,
    pub selected_row: u8,
    pub presentation_state: u16,
    pub requested_presentation_state: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetHitResult {
    pub inside: bool,
    pub activated: bool,
    pub hover_row: u8,
    pub selected_row: u8,
    pub return_ax: u16,
    pub play_select_sound: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ship3dTargetTextSegment {
    TargetList,
    GameData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetDrawCommand {
    pub row_index: usize,
    pub string_segment: Ship3dTargetTextSegment,
    pub string_offset: u16,
    pub x: u16,
    pub y: u16,
    pub color: u8,
    pub measured_width: u16,
    pub extra_entry: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ship3dTargetDrawResult {
    pub commands: Vec<Ship3dTargetDrawCommand>,
    pub final_hover_counter: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceState {
    pub selected_choice: u16,
    pub hud_flags: u8,
    pub handler_phase: u8,
    pub requested_presentation_state: u16,
    pub hold_ticks: u16,
    pub target_y: u16,
    pub target_layout_preserve_widths: bool,
    pub target_layout_center_x: u16,
    pub target_layout_extra_entry: bool,
    pub interpolation_duration_ticks: u8,
    pub interpolation_current_tick: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceGates {
    pub c2_presentation_gate: bool,
    pub left_motion_gate: bool,
    pub right_motion_gate: bool,
    pub menu_gate: bool,
    pub sound_gate: bool,
    pub presentation_active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavChoiceInput {
    pub gate_value: u16,
    pub dynamic_axis: u16,
    pub mouse_x: u16,
    pub mouse_y: u16,
    pub activate: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceResult {
    pub gated: bool,
    pub reset_palette_range: bool,
    pub hovered_choice: Option<u8>,
    pub highlighted_palette_index: Option<u8>,
    pub committed_choice: Option<u8>,
    pub dispatched_choice: Option<u8>,
    pub play_select_sound: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceHandlerEffect {
    pub deferred_record_type: Option<u16>,
    pub deferred_record_related: Option<u16>,
    pub cleared_handler_phase: bool,
    pub ran_layout_prepass: bool,
    pub copied_layout_rect_snapshot: bool,
    pub adjusted_target_records: bool,
    pub phase_gate_blocked: bool,
    pub cleared_selected_choice: bool,
    pub cleared_hud_target_list_flag: bool,
    pub load_snd_bank_path: Option<u16>,
    pub load_voc_path: Option<u16>,
    pub start_voc_playback: bool,
    pub reset_interpolation_tick: bool,
    pub rebuilt_target_records: bool,
    pub set_input_gate_b: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceHandler4State {
    pub layout_rect_snapshot: [u16; SHIP_3D_INTERPOLATION_WORDS],
    pub menu_gate: bool,
    pub secondary_menu_gate: bool,
    pub voc_enabled: bool,
    pub voc_stream_phase: u8,
    pub tablo2_voc_active: bool,
    pub tablo2_voc_reset_gate: bool,
    pub active_target_list_offset: u16,
    pub shared_motion_gate: bool,
    pub left_motion_gate: bool,
    pub right_motion_gate: bool,
    pub sound_gate: u8,
    pub target_activate_flag: bool,
    pub target_activate_secondary_flag: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationSequenceState {
    pub exit_pending: bool,
    pub sequence_active: bool,
    pub opening: bool,
    pub interpolation_duration_ticks: u8,
    pub framebuffer_dirty: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationSequenceEffect {
    pub ran_temp_snd_setup: bool,
    pub ran_procedural_update: bool,
    pub blocked_by_presentation_active: bool,
    pub copied_framebuffer: bool,
    pub interpolation_active: bool,
    pub queried_target_list: bool,
    pub armed_exit_pending: bool,
    pub armed_opening_exit: bool,
    pub final_reset_pending: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProceduralUpdateState {
    pub hud_flags: u16,
    pub angle: u16,
    pub mouse_x: u16,
    pub mouse_y: u16,
    pub hold_ticks: u16,
    pub nav_timer: u16,
    pub mouse_delta_accumulator: u16,
    pub mouse_button_state: u16,
    pub mouse_sector: u16,
    pub rotation_direction_positive: bool,
    pub projection_angle: u16,
    pub rotation_offset: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProceduralUpdateEffect {
    pub cleared_hud_active_flag: bool,
    pub initialized_nav_timer: bool,
    pub applied_hud_rotation: bool,
    pub adjusted_target_list_mouse: bool,
    pub auto_rotated_angle: bool,
    pub updated_projection_angle: bool,
    pub mouse_set_position: Option<(u16, u16)>,
    pub carry_set: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dAngleTableEntry {
    pub cosine: i16,
    pub sine: i16,
}

/// The recovered ship-3D rotation trig table at `DS:0x4F45` in BLOODPRG.EXE:
/// 180 `(cosine, sine)` pairs, one per **2 degrees** (index 45 = 90°, 90 = 180°,
/// wrapping at 180), stored at Q14 amplitude `0x4000`. `matrix_pair_for_angle`
/// doubles each value to Q15 before the fixed-point matrix math. Angle words
/// (`DS:0x2F71` etc.) index this table directly. Verified byte-exact against the
/// binary by `tests::angle_table_matches_binary`.
#[rustfmt::skip]
pub const SHIP_3D_ANGLE_TABLE: [Ship3dAngleTableEntry; 180] = {
    const fn e(cosine: i16, sine: i16) -> Ship3dAngleTableEntry {
        Ship3dAngleTableEntry { cosine, sine }
    }
    [
        e(16384, 0), e(16374, 571), e(16344, 1142), e(16294, 1712), e(16224, 2280), e(16135, 2845),
        e(16025, 3406), e(15897, 3963), e(15749, 4516), e(15582, 5062), e(15395, 5603), e(15190, 6137),
        e(14967, 6663), e(14725, 7182), e(14466, 7691), e(14188, 8191), e(13894, 8682), e(13582, 9161),
        e(13254, 9630), e(12910, 10086), e(12550, 10531), e(12175, 10963), e(11785, 11381), e(11381, 11785),
        e(10963, 12175), e(10531, 12550), e(10086, 12910), e(9630, 13254), e(9161, 13582), e(8682, 13894),
        e(8192, 14188), e(7691, 14466), e(7182, 14725), e(6663, 14967), e(6137, 15190), e(5603, 15395),
        e(5062, 15582), e(4516, 15749), e(3963, 15897), e(3406, 16025), e(2845, 16135), e(2280, 16224),
        e(1712, 16294), e(1142, 16344), e(571, 16374), e(0, 16384), e(-571, 16374), e(-1142, 16344),
        e(-1712, 16294), e(-2280, 16224), e(-2845, 16135), e(-3406, 16025), e(-3963, 15897), e(-4516, 15749),
        e(-5062, 15582), e(-5603, 15395), e(-6137, 15190), e(-6663, 14967), e(-7182, 14725), e(-7691, 14466),
        e(-8191, 14188), e(-8682, 13894), e(-9161, 13582), e(-9630, 13254), e(-10086, 12910), e(-10531, 12550),
        e(-10963, 12175), e(-11381, 11785), e(-11785, 11381), e(-12175, 10963), e(-12550, 10531), e(-12910, 10086),
        e(-13254, 9630), e(-13582, 9161), e(-13894, 8682), e(-14188, 8192), e(-14466, 7691), e(-14725, 7182),
        e(-14967, 6663), e(-15190, 6137), e(-15395, 5603), e(-15582, 5062), e(-15749, 4516), e(-15897, 3963),
        e(-16025, 3406), e(-16135, 2845), e(-16224, 2280), e(-16294, 1712), e(-16344, 1142), e(-16374, 571),
        e(-16384, 0), e(-16374, -571), e(-16344, -1142), e(-16294, -1712), e(-16224, -2280), e(-16135, -2845),
        e(-16025, -3406), e(-15897, -3963), e(-15749, -4516), e(-15582, -5062), e(-15395, -5603), e(-15190, -6137),
        e(-14967, -6663), e(-14725, -7182), e(-14466, -7691), e(-14188, -8191), e(-13894, -8682), e(-13582, -9161),
        e(-13254, -9630), e(-12910, -10086), e(-12550, -10531), e(-12175, -10963), e(-11785, -11381), e(-11381, -11785),
        e(-10963, -12175), e(-10531, -12550), e(-10086, -12910), e(-9630, -13254), e(-9161, -13582), e(-8682, -13894),
        e(-8192, -14188), e(-7691, -14466), e(-7182, -14725), e(-6663, -14967), e(-6137, -15190), e(-5603, -15395),
        e(-5062, -15582), e(-4516, -15749), e(-3963, -15897), e(-3406, -16025), e(-2845, -16135), e(-2280, -16224),
        e(-1712, -16294), e(-1142, -16344), e(-571, -16374), e(0, -16384), e(571, -16374), e(1142, -16344),
        e(1712, -16294), e(2280, -16224), e(2845, -16135), e(3406, -16025), e(3963, -15897), e(4516, -15749),
        e(5062, -15582), e(5603, -15395), e(6137, -15190), e(6663, -14967), e(7182, -14725), e(7691, -14466),
        e(8191, -14188), e(8682, -13894), e(9161, -13582), e(9630, -13254), e(10086, -12910), e(10531, -12550),
        e(10963, -12175), e(11381, -11785), e(11785, -11381), e(12175, -10963), e(12550, -10531), e(12910, -10086),
        e(13254, -9630), e(13582, -9161), e(13894, -8682), e(14188, -8192), e(14466, -7691), e(14725, -7182),
        e(14967, -6663), e(15190, -6137), e(15395, -5603), e(15582, -5062), e(15749, -4516), e(15897, -3963),
        e(16025, -3406), e(16135, -2845), e(16224, -2280), e(16294, -1712), e(16344, -1142), e(16374, -571),
    ]
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dMatrixAngles {
    pub angle_2f71: u16,
    pub projection_angle_2f6d: u16,
    pub angle_2f6f: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionMatrix {
    pub terms: [i32; 9],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionPoint {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionOrigin {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectedPoint {
    pub x: u16,
    pub y: u16,
    pub depth: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionViewport {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectedPixel {
    pub offset: usize,
    pub shade: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dObjectSpriteDescriptor {
    pub flags: u16,
    pub source_width: u16,
    pub source_height: u16,
    pub draw_x: u16,
    pub draw_y: u16,
    pub extent_width: u16,
    pub extent_height: u16,
    pub committed_draw_x: u16,
    pub committed_draw_y: u16,
    pub committed_extent_width: u16,
    pub committed_extent_height: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dObjectSpriteProjection {
    pub projected: Ship3dProjectedPoint,
    pub depth_scale: u16,
    pub scaled_width: u16,
    pub scaled_height: u16,
    pub draw_x: u16,
    pub draw_y: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dSpriteSlotUpdateEffect {
    pub ran: bool,
    pub marked_dirty: bool,
    pub updated_position: bool,
    pub updated_extent: bool,
    pub cleared_extent_changed_flag: bool,
    pub committed_geometry: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ship3dDirtyRectList {
    pub rects: Vec<Ship3dProjectionViewport>,
    pub sentinel: u16,
}

impl Default for Ship3dDirtyRectList {
    fn default() -> Self {
        Self {
            rects: Vec::new(),
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dDirtyRectSnapshotEffect {
    pub ran: bool,
    pub wrote_clip_rect: bool,
    pub wrote_sentinel: bool,
    pub cleared_snapshot_flag: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dSpriteSlotRenderCommand {
    pub slot_index: usize,
    pub dispatch_index: u8,
    pub destination_remap_mode: u8,
    pub flip_x: bool,
    pub flip_y: bool,
    pub slot_rect: Ship3dProjectionViewport,
    pub dirty_rect: Ship3dProjectionViewport,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTempSndState {
    pub trigger: bool,
    pub auxiliary_trigger: bool,
    pub phase: u8,
    pub sequence_active: bool,
    pub plane_copy_enabled: bool,
    pub scene_selector: u16,
    pub hold_ticks: u16,
    pub fullscreen_refresh: bool,
    pub setup_flag_a: bool,
    pub setup_flag_b: bool,
    pub viewport_descriptor: [u16; 8],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTempSndEffect {
    pub ran: bool,
    pub selected_callback_offset: Option<u16>,
    pub next_phase: Option<u8>,
    pub load_snd_bank_path: Option<u16>,
    pub restore_snd_bank_path: Option<u16>,
    pub preserved_mouse_position: bool,
    pub reset_callback_bank_gate: bool,
    pub called_presentation_callback: bool,
    pub reset_hold_ticks: bool,
    pub wrote_viewport_descriptor: bool,
    pub sequence_branch: bool,
    pub non_sequence_branch: bool,
    pub temporarily_disabled_plane_copy: bool,
    pub enabled_plane_copy: bool,
    pub reset_scene_selector: bool,
    pub reset_setup_flags: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavigationFinalResetState {
    pub exit_pending: bool,
    pub opening: bool,
    pub hud_flags: u16,
    pub nav_choice_hold_ticks: u16,
    pub nav_choice_timer: u16,
    pub post_reset_gate: bool,
    pub navigation_gate: bool,
    pub dialogue_state: u16,
    pub scene_band_top: u16,
    pub scene_selector: u16,
    pub active_record: u16,
    pub presentation_gate: bool,
    pub pending_state_byte: bool,
    pub subtitle_gate: bool,
    pub presentation_defer_active: bool,
    pub secondary_presentation_defer_active: bool,
    pub plane_copy_enabled: bool,
    pub sequence_active: bool,
    pub status_flags: u8,
    pub secondary_status_flag: bool,
    pub dirty_marker: u8,
    pub scroll_value: u16,
    pub scroll_mode: u16,
}

impl Default for Ship3dNavigationFinalResetState {
    fn default() -> Self {
        Self {
            exit_pending: false,
            opening: false,
            hud_flags: 0,
            nav_choice_hold_ticks: 0,
            nav_choice_timer: 0,
            post_reset_gate: false,
            navigation_gate: false,
            dialogue_state: 0,
            scene_band_top: 0,
            scene_selector: 0,
            active_record: 0,
            presentation_gate: false,
            pending_state_byte: false,
            subtitle_gate: false,
            presentation_defer_active: false,
            secondary_presentation_defer_active: false,
            plane_copy_enabled: false,
            sequence_active: false,
            status_flags: 0,
            secondary_status_flag: false,
            dirty_marker: 0,
            scroll_value: 0,
            scroll_mode: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationFinalResetEffect {
    pub ran: bool,
    pub reentered_active_sequence: bool,
    pub cleared_dialogue_state: bool,
    pub reset_hud_state: bool,
    pub reset_presentation_gates: bool,
    pub reset_sequence_flags: bool,
    pub reset_status_flags: bool,
    pub copied_backbuffer_restore_block: bool,
    pub cleared_overlay_scratch: bool,
    pub reset_scroll_state: bool,
    pub called_render_clear: bool,
    pub called_input_reset: bool,
    pub called_target_cleanup: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationRuntimeRecord {
    pub offset: u16,
    pub kind_flags: u16,
    pub state_flags: u8,
    pub counter_link: u16,
    pub related_target: u16,
    pub source_parent: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationSourceEntry {
    pub record_offset: u16,
    pub entry_kind: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dPositionRecord {
    pub offset: u16,
    pub kind_flags: u16,
    /// Selector-0x11 parent/reference link. `None` represents the binary's
    /// `0xffff` sentinel, which falls back to the named arche object.
    pub parent_link: Option<u16>,
    pub kind100_match_word: Option<u16>,
    pub kind100_relation_word: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dPositionField {
    pub offset: u16,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dRecordStateSlot {
    pub opcode: u16,
    pub operand: u16,
    pub aux_word: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dC1DestinationWrite {
    pub destination_record_offset: u16,
    pub slot: Ship3dRecordStateSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavigationTriggerState {
    pub trigger_active: bool,
    pub current_target: u16,
    pub requested_presentation_state: u16,
    pub hud_flags: u8,
    pub interpolation_duration_ticks: u8,
    pub interpolation_current_tick: u8,
    pub target_query_mode: bool,
    pub layout_rect_snapshot: [u16; SHIP_3D_INTERPOLATION_WORDS],
    pub sequence_active: bool,
    pub scene_band_top: u16,
    pub render_clip_top: u16,
    pub render_clip_bottom: u16,
    pub active_dialogue_record: u16,
    pub closing: bool,
    pub depth_step: u8,
}

impl Default for Ship3dNavigationTriggerState {
    fn default() -> Self {
        Self {
            trigger_active: false,
            current_target: 0,
            requested_presentation_state: 0,
            hud_flags: 0,
            interpolation_duration_ticks: 0,
            interpolation_current_tick: 0,
            target_query_mode: false,
            layout_rect_snapshot: [0; SHIP_3D_INTERPOLATION_WORDS],
            sequence_active: false,
            scene_band_top: 0,
            render_clip_top: 0,
            render_clip_bottom: SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM,
            active_dialogue_record: 0,
            closing: false,
            depth_step: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationTriggerEffect {
    pub candidate_records: Vec<u16>,
    pub copied_pending_presentation_state: bool,
    pub incremented_counter_record: Option<u16>,
    pub deferred_record_type: Option<u16>,
    pub deferred_record_related: Option<u16>,
    pub candidate_handler_record: Option<u16>,
    pub opened_target_list: bool,
    pub reset_interpolation_tick: bool,
    pub ran_layout_prepass: bool,
    pub copied_layout_x_and_width: bool,
    pub cleared_trigger: bool,
    pub started_sequence: bool,
    pub set_scene_band: bool,
    pub restored_render_clip: bool,
    pub cleared_active_dialogue_record: bool,
    pub requested_closing: bool,
}

pub fn update_ship_3d_transition_state(state: &mut Ship3dTransitionState, random_gate_zero: bool) {
    if !state.transition_armed {
        if state.hold_ticks > SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD {
            state.depth_step = SHIP_3D_TRANSITION_OPEN_STEP;
            state.opening = true;
            state.transition_armed = true;
        }
        return;
    }

    if state.hold_ticks == 0 {
        start_closing_transition(state);
        return;
    }

    if !state.opening && random_gate_zero {
        start_closing_transition(state);
    }
}

pub fn step_ship_3d_depth_scroll(state: &mut Ship3dDepthState) {
    if state.opening {
        if state.depth_offset == SHIP_3D_MAX_DEPTH_OFFSET {
            state.opening = false;
            return;
        }

        let next = add_to_low_byte(state.depth_offset, state.depth_step);
        state.depth_offset = if (next as i16) < SHIP_3D_MAX_DEPTH_OFFSET as i16 {
            next
        } else {
            SHIP_3D_MAX_DEPTH_OFFSET
        };
        return;
    }

    if !state.closing {
        return;
    }

    if state.depth_offset == 0 {
        state.closing = false;
        return;
    }

    let next_low = (state.depth_offset as u8).wrapping_sub(state.depth_step);
    state.depth_offset = if next_low & 0x80 == 0 {
        (state.depth_offset & 0xff00) | next_low as u16
    } else {
        0
    };
}

pub fn copy_ship_3d_plane_bands(
    dest: &mut [u8],
    video_segment: &[u8],
    depth_offset: u16,
    plane_copy_enabled: bool,
    scroll_mode: u16,
) -> Option<Ship3dPlaneBandCopy> {
    if !plane_copy_enabled {
        return None;
    }

    let byte_count = ship_3d_plane_band_byte_count(depth_offset);
    if byte_count > SHIP_3D_PLANE_PAGE_BYTES {
        return None;
    }

    let first_source_start =
        SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + (SHIP_3D_PLANE_PAGE_BYTES - byte_count);
    let first_source_end = first_source_start.checked_add(byte_count)?;
    let second_source_start = SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET;
    let second_source_end = second_source_start.checked_add(byte_count)?;
    let second_dest_start = SHIP_3D_PLANE_DEST_BYTES.checked_sub(byte_count)?;
    let second_dest_end = second_dest_start.checked_add(byte_count)?;

    let first_source = video_segment.get(first_source_start..first_source_end)?;
    let second_source = video_segment.get(second_source_start..second_source_end)?;
    dest.get_mut(0..byte_count)?.copy_from_slice(first_source);
    dest.get_mut(second_dest_start..second_dest_end)?
        .copy_from_slice(second_source);

    Some(Ship3dPlaneBandCopy {
        row_count: byte_count / SHIP_3D_PLANE_ROW_BYTES,
        byte_count,
        first_source_start,
        first_dest_start: 0,
        second_source_start,
        second_dest_start,
        new_scroll_value: (scroll_mode != SHIP_3D_SCROLL_MODE_HOLD)
            .then(|| ship_3d_scroll_value(depth_offset)),
    })
}

pub fn step_ship_3d_interpolation_gate(
    gate: &mut Ship3dInterpolationGate,
    source: [u16; SHIP_3D_INTERPOLATION_WORDS],
    dest: [u16; SHIP_3D_INTERPOLATION_WORDS],
) -> Option<Ship3dInterpolationStep> {
    if gate.duration_ticks == gate.current_tick {
        return Some(Ship3dInterpolationStep::Complete);
    }

    if gate.duration_ticks == 0 {
        return None;
    }

    gate.current_tick = gate.current_tick.wrapping_add(1);
    let mut interpolated = [0u16; SHIP_3D_INTERPOLATION_WORDS];
    for index in 0..SHIP_3D_INTERPOLATION_WORDS {
        let delta = source[index].wrapping_sub(dest[index]) as i16;
        let quotient = checked_i16_div_i8_to_i8(delta, gate.duration_ticks as i8)?;
        let step = (quotient as i16).wrapping_mul(gate.current_tick as i8 as i16);
        interpolated[index] = dest[index].wrapping_add(step as u16);
    }
    Some(Ship3dInterpolationStep::Active(interpolated))
}

pub fn layout_ship_3d_target_list(
    measured_label_widths: &[u16],
    center_x: u16,
    has_extra_entry: bool,
) -> Ship3dTargetListLayout {
    let mut max_label_width = if has_extra_entry {
        SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH
    } else {
        SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH
    };
    let mut height_accumulator = if has_extra_entry {
        SHIP_3D_TARGET_LAYOUT_EXTRA_HEIGHT
    } else {
        0
    };

    for width in measured_label_widths {
        if *width >= max_label_width {
            max_label_width = *width;
        }
        height_accumulator = height_accumulator.wrapping_add(SHIP_3D_TARGET_LAYOUT_ROW_STEP);
    }

    let width = max_label_width.wrapping_add(SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING);
    let height = height_accumulator.wrapping_add(SHIP_3D_TARGET_LAYOUT_HEIGHT_PADDING);
    let x = center_x.wrapping_sub(width >> 1);
    let y = SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT.wrapping_sub(height) >> 1;

    Ship3dTargetListLayout {
        x,
        y,
        width,
        height,
        max_label_width,
        label_count: measured_label_widths.len(),
        has_extra_entry,
        selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
    }
}

pub fn hit_test_ship_3d_target_list(
    state: &mut Ship3dTargetHitState,
    layout: Ship3dTargetListLayout,
    mouse_x: u16,
    mouse_y: u16,
    activate: bool,
) -> Option<Ship3dTargetHitResult> {
    state.hover_row = 0;
    state.selected_row = 0;
    let mut inside = false;
    let mut activated = false;
    let mut play_select_sound = false;

    if signed_i16(mouse_x) >= signed_i16(layout.x) {
        let right = layout.x.wrapping_add(layout.width);
        if signed_i16(mouse_x) <= signed_i16(right) {
            let row_origin = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
            let row_offset = mouse_y.wrapping_sub(row_origin);
            if signed_i16(row_offset) >= 0 {
                let hit_height = layout
                    .height
                    .wrapping_sub(SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET);
                if signed_i16(row_offset) < signed_i16(hit_height) {
                    let row =
                        checked_u16_div_u8_to_u8(row_offset, SHIP_3D_TARGET_LAYOUT_ROW_STEP as u8)?
                            .wrapping_add(1);
                    state.hover_row = row;
                    inside = true;

                    if state.presentation_state != SHIP_3D_TARGET_HOVER_PRESENTATION_MODE {
                        state.presentation_state = 0;
                        state.requested_presentation_state = SHIP_3D_TARGET_HOVER_PRESENTATION_MODE;
                    }

                    if activate {
                        state.requested_presentation_state =
                            SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE;
                        state.selected_row = row;
                        activated = true;
                        play_select_sound = true;
                    }
                }
            }
        }
    }

    if !inside && state.presentation_state != SHIP_3D_TARGET_IDLE_PRESENTATION_MODE {
        state.presentation_state = 0;
        state.requested_presentation_state = SHIP_3D_TARGET_IDLE_PRESENTATION_MODE;
    }

    let return_ax = (state.selected_row as u8).wrapping_sub(1) as i8 as i16 as u16;
    Some(Ship3dTargetHitResult {
        inside,
        activated,
        hover_row: state.hover_row,
        selected_row: state.selected_row,
        return_ax,
        play_select_sound,
    })
}

pub fn update_ship_3d_nav_choice_dispatch(
    state: &mut Ship3dNavChoiceState,
    gates: Ship3dNavChoiceGates,
    input: Ship3dNavChoiceInput,
) -> Option<Ship3dNavChoiceResult> {
    let mut result = Ship3dNavChoiceResult::default();
    if gates.blocks_nav_choice() {
        result.gated = true;
        return Some(result);
    }

    if state.selected_choice == 0 {
        if input.gate_value > SHIP_3D_NAV_CHOICE_MAX_GATE
            || input.gate_value < SHIP_3D_NAV_CHOICE_MIN_GATE
        {
            return Some(result);
        }

        result.reset_palette_range = true;
        if let Some(choice_index) =
            hit_test_ship_3d_nav_choice(input.dynamic_axis, input.mouse_x, input.mouse_y)?
        {
            let choice = choice_index.wrapping_add(1);
            result.hovered_choice = Some(choice);
            result.highlighted_palette_index =
                Some(SHIP_3D_NAV_CHOICE_PALETTE_FIRST.wrapping_add(choice_index));

            if input.activate {
                state.requested_presentation_state = SHIP_3D_NAV_CHOICE_PRESENTATION_MODE;
                state.selected_choice = choice as u16;
                state.hud_flags |= SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS;
                state.hold_ticks = SHIP_3D_NAV_CHOICE_HOLD_TICKS;
                state.handler_phase = SHIP_3D_NAV_CHOICE_HANDLER_PHASE;
                state.target_y = SHIP_3D_NAV_CHOICE_TARGET_Y_BASE.wrapping_add(
                    (choice as u16 - 1).wrapping_mul(SHIP_3D_NAV_CHOICE_TARGET_Y_STEP),
                );
                state.target_layout_preserve_widths = true;
                state.target_layout_center_x = SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X;
                state.target_layout_extra_entry = true;
                state.interpolation_duration_ticks = SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION;
                result.committed_choice = Some(choice);
                result.play_select_sound = Some(SHIP_3D_NAV_CHOICE_SELECT_SOUND);
            }
        }
    }

    if state.selected_choice != 0 && state.hud_flags & SHIP_3D_NAV_CHOICE_DISPATCH_BLOCK_FLAG == 0 {
        let choice = u8::try_from(state.selected_choice).ok()?;
        if choice == 0 || choice > SHIP_3D_NAV_CHOICE_COUNT {
            return None;
        }
        result.dispatched_choice = Some(choice);
    }

    Some(result)
}

pub fn run_ship_3d_nav_choice_handler_0(
    state: &mut Ship3dNavChoiceState,
    named_honk_object: u16,
) -> Ship3dNavChoiceHandlerEffect {
    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE == 0 {
        return Ship3dNavChoiceHandlerEffect::default();
    }

    state.handler_phase = 0;
    Ship3dNavChoiceHandlerEffect {
        deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
        deferred_record_related: Some(named_honk_object),
        cleared_handler_phase: true,
        ..Ship3dNavChoiceHandlerEffect::default()
    }
}

pub fn run_ship_3d_nav_choice_handler_1(
    state: &mut Ship3dNavChoiceState,
    target_records: &mut [u16],
    interpolation_complete: bool,
    query_selection_ax: u16,
) -> Option<Ship3dNavChoiceHandlerEffect> {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        state.interpolation_current_tick = 0;
        adjust_nav_choice_target_records(target_records);
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        effect.ran_layout_prepass = true;
        effect.adjusted_target_records = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return Some(effect);
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if query_selection_ax == SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN {
        return Some(effect);
    }

    let target_index = usize::from(query_selection_ax);
    let target_record = *target_records.get(target_index)?;
    if target_record != SHIP_3D_TARGET_EXIT_SENTINEL {
        effect.deferred_record_type = Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE);
        effect.deferred_record_related =
            Some(target_record.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        effect.load_snd_bank_path = Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET);
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    Some(effect)
}

pub fn run_ship_3d_nav_choice_handler_2(
    state: &mut Ship3dNavChoiceState,
    special_slots: &[u16],
    target_records: &mut Vec<u16>,
    interpolation_complete: bool,
    query_selection_ax: u16,
) -> Option<Ship3dNavChoiceHandlerEffect> {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        rebuild_nav_choice_special_target_records(special_slots, target_records)?;
        state.interpolation_current_tick = 0;
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        effect.ran_layout_prepass = true;
        effect.rebuilt_target_records = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return Some(effect);
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if query_selection_ax == SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN {
        return Some(effect);
    }

    let target_index = usize::from(query_selection_ax);
    let target_record = *target_records.get(target_index)?;
    if target_record != SHIP_3D_TARGET_EXIT_SENTINEL {
        effect.deferred_record_related =
            Some(target_record.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        effect.set_input_gate_b = true;
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    Some(effect)
}

pub fn run_ship_3d_nav_choice_handler_3(
    state: &mut Ship3dNavChoiceState,
    related_record: u16,
) -> Ship3dNavChoiceHandlerEffect {
    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE == 0 {
        return Ship3dNavChoiceHandlerEffect::default();
    }

    state.handler_phase = 0;
    Ship3dNavChoiceHandlerEffect {
        deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
        deferred_record_related: Some(related_record),
        cleared_handler_phase: true,
        load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
        ..Ship3dNavChoiceHandlerEffect::default()
    }
}

pub fn run_ship_3d_nav_choice_handler_4(
    state: &mut Ship3dNavChoiceState,
    handler_state: &mut Ship3dNavChoiceHandler4State,
    layout_rect: [u16; SHIP_3D_INTERPOLATION_WORDS],
    interpolation_complete: bool,
    query_selection_ax: u16,
) -> Ship3dNavChoiceHandlerEffect {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        state.interpolation_current_tick = 0;
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        handler_state.layout_rect_snapshot = layout_rect;
        effect.ran_layout_prepass = true;
        effect.copied_layout_rect_snapshot = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return effect;
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if signed_i16(query_selection_ax) < 0 {
        return effect;
    }

    match query_selection_ax.to_le_bytes()[0] {
        0 => {
            handler_state.menu_gate = true;
            handler_state.secondary_menu_gate = true;
        }
        1 => {
            if handler_state.voc_enabled {
                handler_state.voc_stream_phase = 0;
                if handler_state.tablo2_voc_active {
                    handler_state.tablo2_voc_active = false;
                    handler_state.active_target_list_offset =
                        SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_OFF_TARGET_LIST_OFFSET;
                } else {
                    handler_state.tablo2_voc_reset_gate = false;
                    handler_state.tablo2_voc_active = true;
                    handler_state.active_target_list_offset =
                        SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_ON_TARGET_LIST_OFFSET;
                    effect.load_voc_path = Some(SHIP_3D_NAV_CHOICE_TABLO2_VOC_PATH_OFFSET);
                    effect.start_voc_playback = true;
                }
            }
        }
        2 => {
            handler_state.shared_motion_gate = true;
            handler_state.left_motion_gate = true;
        }
        3 => {
            handler_state.shared_motion_gate = true;
            handler_state.right_motion_gate = true;
        }
        4 => {
            handler_state.sound_gate = SHIP_3D_NAV_CHOICE_SOUND_GATE_SUPPRESS_TARGETS;
            handler_state.target_activate_flag = false;
            handler_state.target_activate_secondary_flag = false;
        }
        _ => {}
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    effect
}

pub fn run_ship_3d_navigation_sequence_update(
    state: &mut Ship3dNavigationSequenceState,
    presentation_active: bool,
    presentation_defer_active: bool,
    interpolation_complete: bool,
    query_selection_ax: u16,
) -> Ship3dNavigationSequenceEffect {
    let mut effect = Ship3dNavigationSequenceEffect::default();

    let run_active_sequence = if state.exit_pending {
        if state.opening {
            true
        } else {
            effect.final_reset_pending = true;
            return effect;
        }
    } else if state.sequence_active {
        true
    } else {
        if !presentation_defer_active {
            state.exit_pending = true;
            state.opening = true;
            effect.armed_opening_exit = true;
        }
        return effect;
    };

    if !run_active_sequence {
        return effect;
    }

    effect.ran_temp_snd_setup = true;
    effect.ran_procedural_update = true;

    if presentation_active {
        effect.blocked_by_presentation_active = true;
        return effect;
    }

    state.framebuffer_dirty = true;
    effect.copied_framebuffer = true;

    if state.interpolation_duration_ticks != SHIP_3D_NAVIGATION_INTERPOLATION_DURATION {
        return effect;
    }

    if !interpolation_complete {
        effect.interpolation_active = true;
        return effect;
    }

    effect.queried_target_list = true;
    if signed_i16(query_selection_ax) >= 0 {
        state.sequence_active = false;
        state.exit_pending = true;
        effect.armed_exit_pending = true;
    }

    effect
}

pub fn run_ship_3d_procedural_update(
    state: &mut Ship3dProceduralUpdateState,
) -> Ship3dProceduralUpdateEffect {
    let mut effect = Ship3dProceduralUpdateEffect::default();
    let mut angle_double = state.angle.wrapping_mul(2);

    if state.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG != 0 {
        let target_angle = state.hold_ticks >> 1;
        if state.angle == target_angle {
            state.hud_flags ^= SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG;
            state.nav_timer = 0;
            effect.cleared_hud_active_flag = true;
        } else {
            let delta = circular_delta(state.angle, target_angle, SHIP_3D_PROCEDURAL_HALF_TURN);
            let compare_angle = wrap_ring_once(
                state.angle as i32 + delta as i32,
                SHIP_3D_PROCEDURAL_HALF_TURN,
            )
            .wrapping_mul(2);

            if state.nav_timer == 0 {
                state.nav_timer = delta;
                effect.initialized_nav_timer = true;
            }

            let mut angle_step = delta >> 1;
            if angle_step == 0 {
                angle_step = 1;
            }
            let mut mouse_step = delta.wrapping_shl(2);
            state.rotation_direction_positive = true;
            if compare_angle != state.hold_ticks {
                state.rotation_direction_positive = false;
                angle_step = angle_step.wrapping_neg();
                mouse_step = mouse_step.wrapping_neg();
            }

            if signed_i16(state.nav_timer) >= signed_i16(SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD) {
                state.mouse_x = state.mouse_x.wrapping_add(mouse_step);
                state.mouse_delta_accumulator =
                    state.mouse_delta_accumulator.wrapping_add(mouse_step);
            }

            state.angle = wrap_ring_once(
                state.angle as i32 + signed_i16(angle_step) as i32,
                SHIP_3D_PROCEDURAL_HALF_TURN,
            );
            state.mouse_button_state = 0;
            angle_double = state.angle.wrapping_mul(2);
            effect.applied_hud_rotation = true;
        }
    }

    state.mouse_x = wrap_ring_once(
        state.mouse_x as i32 - SHIP_3D_PROCEDURAL_MOUSE_RING as i32,
        SHIP_3D_PROCEDURAL_MOUSE_RING,
    );
    effect.mouse_set_position = Some((
        state
            .mouse_x
            .wrapping_add(SHIP_3D_PROCEDURAL_MOUSE_CENTER_X),
        state.mouse_y,
    ));
    state.mouse_sector = state.mouse_x >> 2;

    if state.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG == 0 {
        let delta = circular_delta(
            angle_double,
            state.mouse_sector,
            SHIP_3D_PROCEDURAL_FULL_TURN,
        );
        if delta > SHIP_3D_PROCEDURAL_CLOSE_ANGLE_THRESHOLD {
            if state.hud_flags & SHIP_3D_PROCEDURAL_TARGET_LIST_FLAG != 0 {
                if delta >= SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD {
                    let mouse_plus_delta = wrap_ring_once(
                        state.mouse_sector as i32 + delta as i32,
                        SHIP_3D_PROCEDURAL_FULL_TURN,
                    );
                    let mut target_sector = angle_double;
                    if mouse_plus_delta == angle_double {
                        target_sector = wrap_ring_once(
                            target_sector as i32 - SHIP_3D_PROCEDURAL_TARGET_LIST_STEP as i32,
                            SHIP_3D_PROCEDURAL_FULL_TURN,
                        );
                    } else {
                        target_sector = wrap_ring_once(
                            target_sector as i32 + SHIP_3D_PROCEDURAL_TARGET_LIST_STEP as i32,
                            SHIP_3D_PROCEDURAL_FULL_TURN,
                        );
                    }
                    state.mouse_x = target_sector.wrapping_shl(2);
                    effect.mouse_set_position = Some((
                        state
                            .mouse_x
                            .wrapping_add(SHIP_3D_PROCEDURAL_MOUSE_CENTER_X),
                        state.mouse_y,
                    ));
                    effect.adjusted_target_list_mouse = true;
                }
            } else {
                let mouse_plus_delta = wrap_ring_once(
                    state.mouse_sector as i32 + delta as i32,
                    SHIP_3D_PROCEDURAL_FULL_TURN,
                );
                let next_sector = if mouse_plus_delta != angle_double {
                    state.rotation_direction_positive = true;
                    wrap_ring_once(
                        state.mouse_sector as i32 - SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP as i32,
                        SHIP_3D_PROCEDURAL_FULL_TURN,
                    )
                } else {
                    state.rotation_direction_positive = false;
                    wrap_ring_once(
                        state.mouse_sector as i32 + SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP as i32,
                        SHIP_3D_PROCEDURAL_FULL_TURN,
                    )
                };
                state.angle = next_sector >> 1;
                effect.auto_rotated_angle = true;
            }
        }
    }

    if state.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG != 0 || effect.auto_rotated_angle {
        state.projection_angle = state.angle;
        state.rotation_offset = state
            .angle
            .wrapping_shl(3)
            .wrapping_sub(SHIP_3D_PROCEDURAL_ROTATION_OFFSET_BIAS);
        state.mouse_x &= SHIP_3D_PROCEDURAL_MOUSE_ALIGN_MASK;
        effect.updated_projection_angle = true;
        effect.carry_set = true;
    }

    state.mouse_x = wrap_ring_once(
        state.mouse_x as i32 - state.rotation_offset as i32,
        SHIP_3D_PROCEDURAL_MOUSE_RING,
    );

    effect
}

pub fn build_ship_3d_projection_matrix(
    angle_table: &[Ship3dAngleTableEntry],
    angles: Ship3dMatrixAngles,
) -> Option<Ship3dProjectionMatrix> {
    let (a_cos, a_sin) = matrix_pair_for_angle(angle_table, angles.angle_2f71)?;
    let (b_cos, b_sin) = matrix_pair_for_angle(angle_table, angles.projection_angle_2f6d)?;
    let (c_cos, c_sin) = matrix_pair_for_angle(angle_table, angles.angle_2f6f)?;

    let b_sin_c_sin = fixed_mul_shift_15(b_sin, c_sin);
    let c_sin_b_cos = fixed_mul_shift_15(c_sin, b_cos);

    Some(Ship3dProjectionMatrix {
        terms: [
            a_cos
                .wrapping_mul(b_cos)
                .wrapping_add(b_sin_c_sin.wrapping_mul(a_sin))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            fixed_mul_shift_15(c_cos, a_sin).wrapping_neg(),
            c_sin_b_cos
                .wrapping_mul(a_sin)
                .wrapping_sub(a_cos.wrapping_mul(b_sin))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            b_sin_c_sin
                .wrapping_mul(a_cos)
                .wrapping_sub(a_sin.wrapping_mul(b_cos))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            fixed_mul_shift_15(c_cos, a_cos).wrapping_neg(),
            b_sin
                .wrapping_mul(a_sin)
                .wrapping_add(c_sin_b_cos.wrapping_mul(a_cos))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            fixed_mul_shift_15(b_sin, c_cos),
            c_sin,
            fixed_mul_shift_15(c_cos, b_cos),
        ],
    })
}

pub fn project_ship_3d_point(
    point: Ship3dProjectionPoint,
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
) -> Option<Ship3dProjectedPoint> {
    let translated = [
        projection_component(point.x, origin.x),
        projection_component(point.y, origin.y),
        projection_component(point.z, origin.z),
    ];

    let depth = projection_dot(
        translated,
        [matrix.terms[6], matrix.terms[7], matrix.terms[8]],
    ) >> SHIP_3D_MATRIX_FIXED_SHIFT;
    if depth <= 0 {
        return None;
    }

    let screen_x = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[0], matrix.terms[1], matrix.terms[2]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_X,
    );
    let screen_y = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[3], matrix.terms[4], matrix.terms[5]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_Y,
    );

    Some(Ship3dProjectedPoint {
        x: screen_x,
        y: screen_y,
        depth: depth as u16,
    })
}

pub fn plot_ship_3d_projected_point(
    depth_buffer: &mut [u8],
    viewport: Ship3dProjectionViewport,
    projected: Ship3dProjectedPoint,
) -> Option<Ship3dProjectedPixel> {
    if signed_i16(projected.x) < signed_i16(viewport.left)
        || signed_i16(projected.x) >= signed_i16(viewport.right)
        || signed_i16(projected.y) < signed_i16(viewport.top)
        || signed_i16(projected.y) >= signed_i16(viewport.bottom)
    {
        return None;
    }

    let offset = ship_3d_projected_point_offset(projected);
    let pixel = depth_buffer.get_mut(offset)?;
    if *pixel != 0 {
        return None;
    }

    let shade = ship_3d_projected_point_shade(projected.depth);
    *pixel = shade;
    Some(Ship3dProjectedPixel { offset, shade })
}

pub fn ship_3d_projected_point_shade(depth: u16) -> u8 {
    SHIP_3D_PROJECTION_SHADE_BASE.wrapping_sub((depth >> SHIP_3D_PROJECTION_SHADE_SHIFT) as u8)
}

pub fn ship_3d_projected_point_offset(projected: Ship3dProjectedPoint) -> usize {
    usize::from(projected.y) * SHIP_3D_PROJECTION_SCREEN_WIDTH + usize::from(projected.x)
}

/// Number of 3D point-cloud records the starfield background is built from.
/// The DOS randomizer loops `cx = 0x3E8` records at `DS:0x2FC1`.
pub const SHIP_3D_POINT_CLOUD_LEN: usize = 0x3e8;

/// The engine's pseudo-random generator (`far 0x01CE:0x0B02` in BLOODPRG.EXE).
///
/// Called as `rng(ax = modulus)` and returns `value % modulus` (for
/// `modulus == 0` it returns the raw 16-bit value). The generator threads a
/// carry chain through two state bytes to build a 16-bit word, XORs it with a
/// fixed 16-bit seed word, then advances the two bytes via an incrementing
/// counter. State lives at `cs:0x0AEE` (`seed_word`), `cs:0x0AF0` (`a`),
/// `cs:0x0AF1` (`b`), `cs:0x0AF2` (`counter`); all are zero in the shipped
/// image (the fields below default to that), but the startup code seeds them,
/// so a live run's sequence is not reproducible from the static image alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloodPrng {
    /// `cs:0x0AEE` — XORed into each result; never mutated by the generator.
    pub seed_word: u16,
    /// `cs:0x0AF0` — low mixing byte, advanced each call.
    pub a: u8,
    /// `cs:0x0AF1` — high mixing byte, advanced each call.
    pub b: u8,
    /// `cs:0x0AF2` — call counter used to advance `a`/`b`.
    pub counter: u8,
}

impl Default for BloodPrng {
    /// The static (unseeded) state from the shipped BLOODPRG.EXE image.
    fn default() -> Self {
        Self {
            seed_word: 0,
            a: 0,
            b: 0,
            counter: 0,
        }
    }
}

impl BloodPrng {
    /// Seed as the DOS routine at `0x2DD3` does: it reads the CMOS RTC seconds
    /// byte (`out 0x70 / in 0x71`) and writes it into both halves of the XOR
    /// seed word (`mov ah,al; mov cs:[0xAEE],ax`), leaving the mixing bytes and
    /// counter at zero. Passing the boot second reproduces that run's stream.
    pub fn seeded_from_rtc_seconds(seconds: u8) -> Self {
        Self {
            seed_word: u16::from(seconds) * 0x0101,
            a: 0,
            b: 0,
            counter: 0,
        }
    }

    /// Advance the generator and return the next value in `0..modulus`
    /// (or the raw 16-bit word when `modulus == 0`). Faithful port of the
    /// `rcr/rcl` carry chain and byte advance at `0x01CE:0x0B02`.
    pub fn next(&mut self, modulus: u16) -> u16 {
        // Build a 16-bit word by threading the carry flag through
        // `rcr bl,1 / rcl ax,1 / rcl bh,1 / rcl ax,1`, eight times, starting
        // from a cleared carry (the DOS code `xor ax,ax` clears CF).
        let mut bl = self.a;
        let mut bh = self.b;
        let mut ax: u16 = 0;
        let mut carry: u16 = 0;
        for _ in 0..8 {
            // rcr bl,1
            let new_carry = u16::from(bl & 1);
            bl = ((carry as u8) << 7) | (bl >> 1);
            carry = new_carry;
            // rcl ax,1
            let new_carry = ax >> 15;
            ax = (ax << 1) | carry;
            carry = new_carry;
            // rcl bh,1
            let new_carry = u16::from(bh >> 7);
            bh = ((bh << 1) | (carry as u8)) & 0xff;
            carry = new_carry;
            // rcl ax,1
            let new_carry = ax >> 15;
            ax = (ax << 1) | carry;
            carry = new_carry;
        }

        ax ^= self.seed_word;

        // Advance the two mixing bytes from the incrementing counter.
        self.counter = self.counter.wrapping_add(1);
        let step = self.counter;
        self.b = self.b.wrapping_sub(step);
        self.a ^= step.rotate_left(1);

        // Range-reduce `ax %= modulus` via the DOS repeated-subtraction loop.
        if modulus != 0 {
            while ax >= modulus {
                ax = ax.wrapping_sub(modulus);
            }
        }
        ax
    }
}

/// Populate the ship-3D starfield point cloud (`ship_3d_point_cloud_randomize`
/// at `0x9B67`). Each of the [`SHIP_3D_POINT_CLOUD_LEN`] records gets random
/// `x`/`y`/`z` words from [`BloodPrng::next`] with modulus `0xFFFF`; the DOS
/// loop `add di,2` after the three `stosw`s leaves each record's fourth word
/// untouched, which the projection scratch reuses per frame.
pub fn randomize_ship_3d_point_cloud(prng: &mut BloodPrng) -> Vec<Ship3dProjectionPoint> {
    (0..SHIP_3D_POINT_CLOUD_LEN)
        .map(|_| Ship3dProjectionPoint {
            x: prng.next(0xffff),
            y: prng.next(0xffff),
            z: prng.next(0xffff),
        })
        .collect()
}

/// Height of the ship-3D point-cloud depth/color buffer in rows. The DOS pixel
/// helper computes `y * 320 + x` into the active page; 200 native rows cover it.
pub const SHIP_3D_PROJECTION_SCREEN_HEIGHT: usize = 200;

/// One plotted starfield pixel and, alongside the returned count, the whole
/// depth-shaded buffer produced by [`render_ship_3d_point_cloud`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ship3dPointCloudRender {
    /// `320 * 200` depth-shaded buffer; `0` means "no point drawn here".
    pub buffer: Vec<u8>,
    /// Number of points that projected in front of the camera and won their
    /// depth-buffer cell (matches the DOS write-once behavior).
    pub plotted: usize,
}

/// Render the full ship-3D starfield background: the batch loop at `0x9A10`.
/// Each point is translated by `origin`, projected through `matrix`, and
/// depth-shaded into a `320 * 200` buffer, skipping points at non-positive
/// depth and cells already claimed by a nearer point (the DOS helper only
/// writes empty depth-buffer pixels). This drives the existing
/// [`project_ship_3d_point`] / [`plot_ship_3d_projected_point`] primitives over
/// the whole cloud instead of a single point.
pub fn render_ship_3d_point_cloud(
    points: &[Ship3dProjectionPoint],
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
    viewport: Ship3dProjectionViewport,
) -> Ship3dPointCloudRender {
    let mut buffer = vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
    let mut plotted = 0usize;
    for &point in points {
        let Some(projected) = project_ship_3d_point(point, origin, matrix) else {
            continue;
        };
        if plot_ship_3d_projected_point(&mut buffer, viewport, projected).is_some() {
            plotted += 1;
        }
    }
    Ship3dPointCloudRender { buffer, plotted }
}

/// The ship-nav HUD band occupies the bottom rows of the 320x200 frame (below the
/// scene band that ends at row 0xA5=165), where the engine draws the grey
/// pyramid-nav grid + central eye-orb. See re/REVERSE.md.
pub const SHIP_3D_HUD_BAND_TOP: usize = 0xA5; // 165

/// The recovered ship-nav HUD pyramid geometry: 32 3D vertices (X,Y,Z, signed
/// fixed-point) copied by `ship_3d_hud_init` (BLOODPRG.EXE @0xB079) from DS:0x5D98
/// (file 0x131B8) into the HUD working area at ship-view entry, then projected by
/// the shared matrix×vector + perspective pipeline. Vertices 16..23 form a linear
/// compass axis; the rest are the pyramid/HUD corners.
///
/// Disassembly-recovered render path (sess 005), the missing transform + draw:
/// - `ship_3d_hud_init` @0xB079: `rep movsd` 0x30=48 dwords (32 verts × 3 words =
///   96 words) from `si=0x5D98` to `di=0x5491` (working copy); then `[0x2795]=0xB3`
///   (the compass *entry angle* — this is the projection angle to use, NOT 0),
///   `[0x279B]=0`, and `[0x2793] |= 8` (HUD gate bit 3).
/// - The compass angle `[0x2795]` animates 0..0xB3 (wraps at 0xB4=180) in
///   `ship_3d_procedural_angle_update` @0x9656.
/// - HUD draw prelude @0xB14A: re-copies 0x10 dwords from `0x5491` into the frame
///   working area `0x5551`, sets the band bounds `[0x5239]=0x23` (35) and
///   `[0x523B]=0xA5` (165) → **the HUD occupies the y=165..200 band (35px)**, then
///   renders via `lcall 0x1CE:0` (the projection/raster segment). So the ship3d HUD
///   is the COMPACT dialogue-mode nav strip; the full-screen star-map nav screen
///   (rows of shaded pyramids) is a SEPARATE view — don't conflate them.
/// - `0x1CE:0` (file 0x22E0) is a `/100` fixed-point perspective helper (called with
///   ax=-50, di=0x5F11 workspace). The pyramid render then dispatches through segment
///   0x299: `lcall 0x299:0x1467` and `lcall 0x299:0x210D` (after `ship_3d_target_
///   record_select` @0xB2BB selects the active target). So the vertex→screen raster
///   lives in seg 0x299; `di=0x6612`/`0x6724` are its record pointers.
/// - `0x299:0x1467` (file 0x43F9) iterates **32-byte records** at `si=0x6212`
///   (indexes by `ax<<5`), emitting to the `di=0x6612` draw list (dword pairs from
///   `[0x5235]`/`[0x5239]` + a 0xFFFF terminator). So the projected pyramid geometry
///   is a 32-byte-record display list; `0x299:0x210D` consumes/rasterises it.
/// - `0x299:0x210D` (file 0x509D) is the **rasteriser**: gated by `gs:[0x5231]`, it
///   walks the display list reading 8-byte segment records (`es:[di]`, `[di+2]`,
///   `[di+4]`, `[di+6]` = endpoints; `di += 8`), computes framebuffer offsets against
///   width 0x140=320, and draws the pyramid edges/spans into `gs:[0x5221]`.
///
/// PIPELINE NOW MAPPED END-TO-END (routine level): hud_init (verts→0x5491, angle
/// 0xB3) → prelude (band y165-200) → 0x1CE:0 (/100 perspective) → 0x299:0x1467
/// (32-byte-record display list @0x6212→0x6612) → 0x299:0x210D (8-byte-segment
/// rasteriser). The `0x1CE:0`/`0x22E0` transform reads a rotation matrix from
/// `0x5251` (byte components via `lodsb`/`cwde`), applies `/100` fixed-point scaling
/// with per-axis scale (`[bp+0x10]`) + offset (`[bp+0x12/14/16]`) params, emitting
/// projected coords — i.e. matrix×vector then perspective, same shape as
/// [`project_ship_3d_point`] but with the HUD's own 0x5251 matrix + 0x5F11 origin.
/// 32-BYTE RECORD LAYOUT (partly decoded from the 0x43F9 loop, stride 0x20):
///   [0] = flags byte (bits 0+1 both set → the record draws); [8],[0xC] = current
///   projected coord dwords; [0x10],[0x14] = previous coords (the loop copies 8→0x10,
///   0xC→0x14 each pass, so the rasteriser's 8-byte segment = prev→cur endpoints).
///   The 8-byte rasteriser records (0x509D) are the {cur,prev} endpoint pairs.
/// MATRIX CONFIRMED: `ship_3d_projection_matrix_build` @0x98B9 builds the 3×3 matrix
/// at DS:0x2F95 from the angle table + angle words 0x2F71/0x2F6D/0x2F6F — i.e. this is
/// exactly [`build_ship_3d_projection_matrix`] (same angle fields). 0x5251 is then a
/// working copy (`rep movsd 0xC0` from 0x5B58). So the rotation half of the HUD
/// projection is ALREADY implemented.
///
/// CORRECTION (was wrong before): `0x1CE:0`/`0x22E0` is NOT the perspective transform.
/// Full decode shows it computes squared distances `Σ(a_i-b_i)²` over records and
/// tracks the minimum — a NEAREST-POINT / hit-test search (which pyramid the cursor is
/// closest to), not a projection. So the actual vertex→screen PROJECTION for the
/// pyramids is still unlocated — it runs before `0x299:0x1467` fills the 0x6212 records
/// with already-projected coords. TODO (next session): find the routine that projects
/// the 0x5491 verts into the 0x6212 display-list records (that IS the missing
/// projection), plus the compass→matrix-angle map; then reimplement + diff vs oracle.
///
/// FURTHER (sess 005): the 0x6212-record builder @0x40D0 (seg 0x299) writes
/// `((flags & 4) | 0x83)` into each record — that is the SPRITE bank dispatch (same
/// formula as `sprite::bank_dispatch_index`). So the 0x6212 records carry sprite-draw
/// dispatch: the HUD pyramids are very likely SPRITES drawn at projected positions,
/// not a pure 3D wireframe. This reframes the render as hybrid (3D-projected placement
/// + sprite blit) and is why single-routine estimates kept being wrong. Genuinely
/// multi-session: needs the projection→position math AND the pyramid sprite source.
pub const SHIP_3D_HUD_PYRAMID_VERTICES: [[i16; 3]; 32] = [
    [0, 2304, 3075],
    [776, 1803, 2820],
    [775, 1546, 2306],
    [517, 1288, 1793],
    [262, 1544, 2308],
    [1034, 2573, 3589],
    [1547, 3088, 4615],
    [2062, 2068, 3076],
    [2829, 3093, 5901],
    [3081, 2840, 6415],
    [4362, 2331, 7186],
    [4359, 1562, 5903],
    [4362, 2327, 5388],
    [4368, 4892, 7956],
    [6159, 3101, 8729],
    [6670, 3875, 9244],
    [0, 1024, 1028],
    [2056, 3080, 3084],
    [4369, 5393, 5397],
    [6425, 7449, 7453],
    [8738, 9762, 9766],
    [10794, 11818, 11822],
    [13107, 14131, 14135],
    [15163, 16187, 16191],
    [7697, 4901, 10016],
    [7959, 2596, 8214],
    [5898, 2334, 7700],
    [8982, 6442, 10532],
    [9753, 6956, 11817],
    [11296, 9008, 13103],
    [60, 0, 36],
    [13323, 8194, 6719],
];

/// Render the pyramid-nav HUD grid into the bottom band of an indexed 320x200
/// framebuffer (the band the exporter otherwise leaves BLACK in dialogue mode).
/// Draws a perspective grid of grey pyramid (triangle) outlines matching the
/// in-game nav HUD verified against the playthrough, with the central eye-orb
/// position marked. FUNCTIONAL render: the exact per-pixel procedural algorithm
/// (BLOODPRG.EXE ship-3D HUD @~0x9656/0xB193, 3D-projected pyramid vertices) is
/// still being decoded; this fills the HUD band with the correct structure.
pub fn render_ship_3d_pyramid_hud(buffer: &mut [u8], grid_color: u8, orb_color: u8) {
    const W: isize = SHIP_3D_PROJECTION_SCREEN_WIDTH as isize; // 320
    const H: isize = SHIP_3D_PROJECTION_SCREEN_HEIGHT as isize; // 200
    let band_top = SHIP_3D_HUD_BAND_TOP as isize; // 165
    let plot = |buf: &mut [u8], x: isize, y: isize, c: u8| {
        if (0..W).contains(&x) && (band_top..H).contains(&y) {
            buf[(y * W + x) as usize] = c;
        }
    };
    // Bresenham line, clipped to the HUD band.
    let line = |buf: &mut [u8], x0: isize, y0: isize, x1: isize, y1: isize, c: u8| {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut err, mut x, mut y) = (dx + dy, x0, y0);
        loop {
            plot(buf, x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    };
    // Perspective grid: rows recede toward a vanishing point up-centre; nearer
    // (lower) rows have larger, wider-spaced pyramids.
    let center_x = W / 2;
    for row in 0..3isize {
        let base_y = band_top + 6 + row * 9; // row baseline
        let half = 4 + row * 2; // pyramid half-width grows toward front
        let apex = base_y - (5 + row * 2); // taller toward front
        let spacing = (half * 2 + 6) as isize;
        let cols = (W / spacing) + 1;
        for col in -(cols / 2)..=(cols / 2) {
            let cx = center_x + col * spacing;
            // pyramid = two slanted edges to the apex + a base line
            line(buffer, cx - half, base_y, cx, apex, grid_color);
            line(buffer, cx + half, base_y, cx, apex, grid_color);
            line(buffer, cx - half, base_y, cx + half, base_y, grid_color);
        }
    }
    // Central eye-orb: a small filled disc centred in the band.
    let orb_cy = band_top + 16;
    let r = 6isize;
    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y <= r * r {
                plot(buffer, center_x + x, orb_cy + y, orb_color);
            }
        }
    }
}

/// Render a complete ship-3D starfield background from real game data: randomize
/// the point cloud from `prng`, build the camera matrix from `angles` using the
/// recovered [`SHIP_3D_ANGLE_TABLE`], and project/depth-shade into a 320x200
/// buffer. Returns `None` only if `angles` index outside the trig table (they
/// are `% 180` in the engine, so any in-range angle succeeds). This is the whole
/// background layer — the sprite slots and HUD compose over it separately.
pub fn render_ship_3d_starfield(
    prng: &mut BloodPrng,
    angles: Ship3dMatrixAngles,
    origin: Ship3dProjectionOrigin,
    viewport: Ship3dProjectionViewport,
) -> Option<Ship3dPointCloudRender> {
    let points = randomize_ship_3d_point_cloud(prng);
    let matrix = build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles)?;
    Some(render_ship_3d_point_cloud(
        &points, origin, matrix, viewport,
    ))
}

pub fn project_ship_3d_object_sprite(
    anchor: Ship3dProjectionPoint,
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
    descriptor: &mut Ship3dObjectSpriteDescriptor,
) -> Option<Ship3dObjectSpriteProjection> {
    if descriptor.flags & SHIP_3D_OBJECT_VISIBLE_FLAG == 0 {
        return None;
    }

    let translated = [
        projection_component(anchor.x, origin.x),
        projection_component(anchor.y, origin.y),
        projection_component(anchor.z, origin.z),
    ];
    let raw_depth = projection_dot(
        translated,
        [matrix.terms[6], matrix.terms[7], matrix.terms[8]],
    ) >> SHIP_3D_MATRIX_FIXED_SHIFT;
    if raw_depth == 0 {
        return None;
    }

    let depth = if raw_depth < 0 {
        raw_depth.wrapping_add(SHIP_3D_OBJECT_DEPTH_WRAP_BIAS)
    } else {
        raw_depth
    };
    if depth == 0 {
        return None;
    }

    let depth_scale = (SHIP_3D_OBJECT_SCALE_NUMERATOR / depth as u32) as u16;
    let screen_x = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[0], matrix.terms[1], matrix.terms[2]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_X,
    );
    let screen_y = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[3], matrix.terms[4], matrix.terms[5]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_Y,
    );
    let scaled_width = scale_ship_3d_object_dimension(descriptor.source_width, depth_scale);
    let scaled_height = scale_ship_3d_object_dimension(descriptor.source_height, depth_scale);
    update_ship_3d_sprite_slot_extent(descriptor, scaled_width, scaled_height);

    let draw_x = screen_x.wrapping_sub(descriptor.extent_width >> 1);
    let draw_y = screen_y.wrapping_sub(descriptor.extent_height >> 1);
    update_ship_3d_sprite_slot_position(descriptor, draw_x, draw_y);

    Some(Ship3dObjectSpriteProjection {
        projected: Ship3dProjectedPoint {
            x: screen_x,
            y: screen_y,
            depth: depth as u16,
        },
        depth_scale,
        scaled_width,
        scaled_height,
        draw_x,
        draw_y,
    })
}

pub fn update_ship_3d_sprite_slot_position(
    descriptor: &mut Ship3dObjectSpriteDescriptor,
    x: u16,
    y: u16,
) -> Ship3dSpriteSlotUpdateEffect {
    let mut effect = Ship3dSpriteSlotUpdateEffect::default();
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_ACTIVE_MASK == 0 {
        return effect;
    }

    effect.ran = true;
    if descriptor.draw_x != x {
        descriptor.flags |= SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
        descriptor.draw_x = x;
        effect.marked_dirty = true;
        effect.updated_position = true;
    }
    if descriptor.draw_y != y {
        descriptor.flags |= SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
        descriptor.draw_y = y;
        effect.marked_dirty = true;
        effect.updated_position = true;
    }
    effect
}

pub fn update_ship_3d_sprite_slot_extent(
    descriptor: &mut Ship3dObjectSpriteDescriptor,
    width: u16,
    height: u16,
) -> Ship3dSpriteSlotUpdateEffect {
    let mut effect = Ship3dSpriteSlotUpdateEffect::default();
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_ACTIVE_MASK == 0 {
        return effect;
    }

    effect.ran = true;
    if width == descriptor.source_width && height == descriptor.source_height {
        if descriptor.flags & SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG != 0 {
            descriptor.flags &= !SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG;
            descriptor.flags |= SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
            effect.marked_dirty = true;
            effect.cleared_extent_changed_flag = true;
        }
        return effect;
    }

    if descriptor.extent_width != width || descriptor.extent_height != height {
        descriptor.flags |=
            SHIP_3D_SPRITE_SLOT_DIRTY_FLAG | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG;
        descriptor.extent_width = width;
        descriptor.extent_height = height;
        effect.marked_dirty = true;
        effect.updated_extent = true;
    }
    effect
}

pub fn commit_ship_3d_sprite_slot_dirty_geometry(
    descriptor: &mut Ship3dObjectSpriteDescriptor,
) -> Ship3dSpriteSlotUpdateEffect {
    let mut effect = Ship3dSpriteSlotUpdateEffect::default();
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG == 0 {
        return effect;
    }

    effect.ran = true;
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG == 0 {
        return effect;
    }

    descriptor.committed_draw_x = descriptor.draw_x;
    descriptor.committed_draw_y = descriptor.draw_y;
    descriptor.committed_extent_width = descriptor.extent_width;
    descriptor.committed_extent_height = descriptor.extent_height;
    effect.committed_geometry = true;
    effect
}

pub fn commit_ship_3d_global_clip_snapshot(
    dirty_rects: &mut Ship3dDirtyRectList,
    snapshot_armed: &mut bool,
    clip: Ship3dProjectionViewport,
) -> Ship3dDirtyRectSnapshotEffect {
    if !*snapshot_armed {
        return Ship3dDirtyRectSnapshotEffect::default();
    }

    dirty_rects.rects.clear();
    dirty_rects.rects.push(clip);
    dirty_rects.sentinel = SHIP_3D_DIRTY_RECT_SENTINEL;
    *snapshot_armed = false;

    Ship3dDirtyRectSnapshotEffect {
        ran: true,
        wrote_clip_rect: true,
        wrote_sentinel: true,
        cleared_snapshot_flag: true,
    }
}

pub fn collect_ship_3d_dirty_sprite_slot_render_commands(
    slots: &mut [Ship3dObjectSpriteDescriptor],
    dirty_rects: &Ship3dDirtyRectList,
    start_index: usize,
    end_index: usize,
) -> Vec<Ship3dSpriteSlotRenderCommand> {
    if dirty_rects.rects.is_empty() || start_index > end_index {
        return Vec::new();
    }

    let mut commands = Vec::new();
    for slot_index in (start_index..=end_index).rev() {
        let Some(slot) = slots.get_mut(slot_index) else {
            continue;
        };
        let flags = slot.flags;

        if flags & SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG != 0 {
            let slot_rect = Ship3dProjectionViewport {
                left: slot.draw_x,
                right: slot.draw_x.wrapping_add(slot.extent_width),
                top: slot.draw_y,
                bottom: slot.draw_y.wrapping_add(slot.extent_height),
            };
            for dirty_rect in &dirty_rects.rects {
                if ship_3d_rects_intersect(slot_rect, *dirty_rect) {
                    commands.push(Ship3dSpriteSlotRenderCommand {
                        slot_index,
                        dispatch_index: ((flags >> 1) & 0x07) as u8,
                        destination_remap_mode: ((flags >> 8) & 0x03) as u8,
                        flip_x: flags & 0x0020 != 0,
                        flip_y: flags & 0x0040 != 0,
                        slot_rect,
                        dirty_rect: *dirty_rect,
                    });
                }
            }
        }

        slot.flags &= !SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
    }

    commands
}

pub fn run_ship_3d_temp_snd_setup(state: &mut Ship3dTempSndState) -> Option<Ship3dTempSndEffect> {
    if !state.trigger {
        return Some(Ship3dTempSndEffect::default());
    }

    let selected_callback_offset =
        SHIP_3D_TEMP_SND_CALLBACK_OFFSETS.get(usize::from(state.phase))?;
    let mut effect = Ship3dTempSndEffect {
        ran: true,
        selected_callback_offset: Some(*selected_callback_offset),
        load_snd_bank_path: Some(SHIP_3D_TEMP_SND_PATH_OFFSET),
        restore_snd_bank_path: Some(SHIP_3D_TB_SND_PATH_OFFSET),
        preserved_mouse_position: true,
        reset_callback_bank_gate: true,
        called_presentation_callback: true,
        reset_hold_ticks: true,
        wrote_viewport_descriptor: true,
        ..Ship3dTempSndEffect::default()
    };

    state.trigger = false;
    state.auxiliary_trigger = false;
    state.phase = next_ship_3d_temp_snd_phase(state.phase);
    effect.next_phase = Some(state.phase);
    state.hold_ticks = 0;
    state.fullscreen_refresh = true;
    state.viewport_descriptor = SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR;

    if state.sequence_active {
        state.plane_copy_enabled = false;
        effect.temporarily_disabled_plane_copy = true;
        state.plane_copy_enabled = true;
        state.scene_selector = SHIP_3D_TEMP_SND_SCENE_SELECTOR_SENTINEL;
        effect.enabled_plane_copy = true;
        effect.reset_scene_selector = true;
        effect.sequence_branch = true;
    } else {
        state.setup_flag_a = false;
        state.setup_flag_b = false;
        effect.reset_setup_flags = true;
        effect.non_sequence_branch = true;
    }

    Some(effect)
}

pub fn run_ship_3d_navigation_final_reset(
    state: &mut Ship3dNavigationFinalResetState,
) -> Ship3dNavigationFinalResetEffect {
    if !state.exit_pending {
        return Ship3dNavigationFinalResetEffect::default();
    }

    if state.opening {
        return Ship3dNavigationFinalResetEffect {
            reentered_active_sequence: true,
            ..Ship3dNavigationFinalResetEffect::default()
        };
    }

    state.hud_flags = SHIP_3D_FINAL_RESET_HUD_FLAGS;
    state.nav_choice_hold_ticks = 0;
    state.nav_choice_timer = SHIP_3D_FINAL_RESET_NAV_TIMER;
    state.post_reset_gate = true;
    state.navigation_gate = true;

    state.dialogue_state = 0;
    state.scene_band_top = 0;
    state.scene_selector = SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL;
    state.active_record = SHIP_3D_FINAL_RESET_ACTIVE_RECORD_SENTINEL;
    state.presentation_gate = false;
    state.exit_pending = false;
    state.pending_state_byte = false;
    state.subtitle_gate = false;
    state.presentation_defer_active = false;
    state.secondary_presentation_defer_active = false;
    state.plane_copy_enabled = false;
    state.sequence_active = false;
    state.status_flags &= SHIP_3D_FINAL_RESET_STATUS_FLAG_MASK;
    state.secondary_status_flag = false;

    state.dirty_marker = SHIP_3D_FINAL_RESET_DIRTY_MARKER;
    state.scroll_value = 0;
    state.scroll_mode = SHIP_3D_FINAL_RESET_SCROLL_MODE;

    Ship3dNavigationFinalResetEffect {
        ran: true,
        cleared_dialogue_state: true,
        reset_hud_state: true,
        reset_presentation_gates: true,
        reset_sequence_flags: true,
        reset_status_flags: true,
        copied_backbuffer_restore_block: true,
        cleared_overlay_scratch: true,
        reset_scroll_state: true,
        called_render_clear: true,
        called_input_reset: true,
        called_target_cleanup: true,
        ..Ship3dNavigationFinalResetEffect::default()
    }
}

pub fn build_ship_3d_navigation_source_records(
    source_entries: &[Ship3dNavigationSourceEntry],
    records: &[Ship3dNavigationRuntimeRecord],
    root_target: u16,
) -> Option<Vec<u16>> {
    let mut source_records = Vec::new();
    append_ship_3d_navigation_source_children(
        source_entries,
        records,
        root_target,
        &mut source_records,
    )?;
    source_records.push(SHIP_3D_TARGET_EXIT_SENTINEL);
    Some(source_records)
}

pub fn build_ship_3d_navigation_candidate_records(
    source_records: &[u16],
    records: &[Ship3dNavigationRuntimeRecord],
    honk_object: u16,
) -> Option<Vec<u16>> {
    let mut candidates = Vec::new();
    for record_offset in source_records {
        if *record_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            return Some(candidates);
        }
        if *record_offset == honk_object {
            continue;
        }

        let record = find_ship_3d_navigation_record(records, *record_offset)?;
        if record.kind_flags == SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE
            && record.state_flags & SHIP_3D_NAVIGATION_RECORD_ACTIVE_FLAG != 0
        {
            candidates.push(*record_offset);
        }
    }
    None
}

pub fn resolve_ship_3d_position_field(
    records: &[Ship3dPositionRecord],
    record_offset: u16,
    arche_object: u16,
    kind100_compare_word: u16,
) -> Option<u16> {
    let mut current_offset = record_offset;
    for _ in 0..records.len().saturating_add(1) {
        let record = find_ship_3d_position_record(records, current_offset)?;
        match record.kind_flags {
            SHIP_3D_OBJECT_KIND_POSITION_KIND100 => {
                let selector = if record.kind100_match_word? == kind100_compare_word {
                    SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH
                } else {
                    SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH
                };
                return ship_3d_record_field(record.offset, record.kind_flags, selector);
            }
            SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8
            | SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10
            | SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40
            | SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200 => {
                return ship_3d_record_field(
                    record.offset,
                    record.kind_flags,
                    SHIP_3D_FIELD_SELECTOR_POSITION,
                );
            }
            kind_flags => {
                let parent_field =
                    vm::vm_field_offset(SHIP_3D_FIELD_SELECTOR_PARENT_LINK, kind_flags)?;
                if parent_field == 0 {
                    return None;
                }
                current_offset = record.parent_link.unwrap_or(arche_object);
            }
        }
    }
    None
}

pub fn ship_3d_position_distance(
    records: &[Ship3dPositionRecord],
    fields: &[Ship3dPositionField],
    first_record_offset: u16,
    second_record_offset: u16,
    arche_object: u16,
    inherited_kind100_compare_word: u16,
) -> Option<u16> {
    let first_record = find_ship_3d_position_record(records, first_record_offset)?;
    let second_record = find_ship_3d_position_record(records, second_record_offset)?;
    let first_field_offset = resolve_ship_3d_distance_position_field(
        records,
        first_record,
        second_record,
        arche_object,
        inherited_kind100_compare_word,
    )?;
    let second_field_offset = resolve_ship_3d_distance_position_field(
        records,
        second_record,
        first_record,
        arche_object,
        inherited_kind100_compare_word,
    )?;
    let first_field = find_ship_3d_position_field(fields, first_field_offset)?;
    let second_field = find_ship_3d_position_field(fields, second_field_offset)?;
    ship_3d_position_field_distance(first_field, second_field)
}

pub fn ship_3d_position_field_distance(
    first: Ship3dPositionField,
    second: Ship3dPositionField,
) -> Option<u16> {
    let dx = binary_abs_word_diff(first.x, second.x) as i16 as i32 as u32;
    let dy = binary_abs_word_diff(first.y, second.y) as i16 as i32 as u32;
    let squared = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
    ship_3d_binary_sqrt(squared)
}

pub fn ship_3d_binary_sqrt(value: u32) -> Option<u16> {
    let mut ax = value as u16;
    let mut dx = (value >> 16) as u16;
    let original_ax = ax;
    let original_dx = dx;

    let mut bx = if dx != 0 {
        if dx & 0xff00 != 0 {
            if dx >= 0xfffe {
                return Some(ax);
            }
            0xffff
        } else {
            0x0fff
        }
    } else {
        if ax == 0 {
            return Some(ax);
        }
        if ax & 0xff00 != 0 { 0x00ff } else { 0x000f }
    };

    loop {
        let dividend = ((dx as u32) << 16) | ax as u32;
        let quotient = dividend / bx as u32;
        if quotient > u16::MAX as u32 {
            return None;
        }
        let (sum, carry) = (quotient as u16).overflowing_add(bx);
        let candidate = (sum >> 1) | if carry { 0x8000 } else { 0 };
        if candidate >= bx {
            return Some(candidate);
        }
        bx = candidate;
        ax = original_ax;
        dx = original_dx;
    }
}

pub fn ship_3d_object_table_bit_is_set(
    object_table_records: &[u16],
    bitset_base: &[u8],
    object_record_offset: u16,
) -> Option<bool> {
    let object_index = object_table_records
        .iter()
        .position(|record| *record == object_record_offset)?;
    let field_offset =
        vm::vm_field_offset(SHIP_3D_SOURCE_BITSET_SELECTOR, SHIP_3D_SOURCE_BITSET_KIND)? as usize;
    let byte_offset = field_offset.checked_add(object_index >> 3)?;
    let value = *bitset_base.get(byte_offset)?;
    let mask = vm::bit_flag_mask((object_index & 7) as u8);
    Some(value & mask != 0)
}

/// `source_list_bytes` starts at the binary's `DS:0x6886` scratch list. Kind-2
/// tests use the post-`lodsw` cursor for the current source record as the bitset
/// base before applying helper `0x6210`'s selector-5 offset.
pub fn select_ship_3d_c1_source_record(
    source_records: &[u16],
    records: &[Ship3dNavigationRuntimeRecord],
    object_table_records: &[u16],
    source_list_bytes: &[u8],
    operand_record_offset: u16,
    operand_state_flags: u8,
) -> Option<Option<u16>> {
    for (source_index, record_offset) in source_records.iter().enumerate() {
        if *record_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            return Some(None);
        }

        let record = find_ship_3d_navigation_record(records, *record_offset)?;
        match record.kind_flags {
            SHIP_3D_C1_SOURCE_KIND_BITSET => {
                let bitset_cursor = source_index.checked_add(1)?.checked_mul(2)?;
                let bitset_base = source_list_bytes.get(bitset_cursor..)?;
                if ship_3d_object_table_bit_is_set(
                    object_table_records,
                    bitset_base,
                    operand_record_offset,
                )? {
                    return Some(Some(record.offset));
                }
            }
            SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG => {
                if operand_state_flags & SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG != 0 {
                    return Some(Some(record.offset));
                }
            }
            _ => {}
        }
    }

    None
}

pub fn resolve_ship_3d_c1_kind10_destination_record(
    target_record_offset: u16,
    target_kind_flags: u16,
) -> Option<u16> {
    if target_kind_flags != SHIP_3D_C1_KIND10_RECORD_KIND {
        return None;
    }
    vm::vm_field_offset(
        SHIP_3D_C1_DESTINATION_SELECTOR,
        SHIP_3D_C1_KIND10_RECORD_KIND,
    )
    .map(|field| target_record_offset.wrapping_add(field))
}

pub fn write_ship_3d_c1_kind10_destination_slot(
    target_record_offset: u16,
    target_kind_flags: u16,
    destination_slot: &mut Ship3dRecordStateSlot,
    operand_record_offset: u16,
) -> Option<Option<Ship3dC1DestinationWrite>> {
    let destination_record_offset =
        resolve_ship_3d_c1_kind10_destination_record(target_record_offset, target_kind_flags)?;
    if destination_slot.opcode != 0 {
        return Some(None);
    }

    *destination_slot = Ship3dRecordStateSlot {
        opcode: SHIP_3D_C1_RECORD_STATE_OPCODE,
        operand: operand_record_offset,
        aux_word: SHIP_3D_C1_RECORD_STATE_AUX_WORD,
    };
    Some(Some(Ship3dC1DestinationWrite {
        destination_record_offset,
        slot: *destination_slot,
    }))
}

pub fn run_ship_3d_navigation_trigger_prelude(
    state: &mut Ship3dNavigationTriggerState,
    records: &[Ship3dNavigationRuntimeRecord],
    source_records: &[u16],
    honk_object: u16,
    ark_object: u16,
    pending_presentation_state: u16,
    layout_rect: [u16; SHIP_3D_INTERPOLATION_WORDS],
) -> Option<Ship3dNavigationTriggerEffect> {
    let mut effect = Ship3dNavigationTriggerEffect::default();
    if !state.trigger_active {
        return Some(effect);
    }

    state.requested_presentation_state = pending_presentation_state;
    effect.copied_pending_presentation_state = true;

    let current_record = find_ship_3d_navigation_record(records, state.current_target)?;
    effect.incremented_counter_record = Some(
        if current_record.kind_flags & SHIP_3D_NAVIGATION_REDIRECT_COUNTER_FLAG != 0 {
            current_record.counter_link
        } else {
            current_record.offset
        },
    );

    let candidate_records =
        build_ship_3d_navigation_candidate_records(source_records, records, honk_object)?;
    effect.candidate_records = candidate_records;

    let mut opened_target_list = true;
    for candidate_record_offset in &effect.candidate_records {
        let candidate_record = find_ship_3d_navigation_record(records, *candidate_record_offset)?;
        if current_record.state_flags & SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG == 0
            && candidate_record.related_target != state.current_target
        {
            continue;
        }

        if ark_object != state.current_target && candidate_record.related_target == ark_object {
            break;
        }

        effect.deferred_record_type = Some(SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE);
        effect.deferred_record_related = Some(*candidate_record_offset);
        effect.candidate_handler_record =
            Some(candidate_record_offset.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        opened_target_list = false;
        break;
    }

    if opened_target_list {
        state.hud_flags |= SHIP_3D_NAVIGATION_TARGET_LIST_FLAG;
        state.interpolation_current_tick = 0;
        state.interpolation_duration_ticks = SHIP_3D_NAVIGATION_INTERPOLATION_DURATION;
        state.target_query_mode = false;
        state.layout_rect_snapshot[0] = layout_rect[0];
        state.layout_rect_snapshot[2] = layout_rect[2];
        effect.opened_target_list = true;
        effect.reset_interpolation_tick = true;
        effect.ran_layout_prepass = true;
        effect.copied_layout_x_and_width = true;
    }

    state.trigger_active = false;
    state.sequence_active = true;
    state.scene_band_top = SHIP_3D_NAVIGATION_SCENE_BAND_TOP;
    state.render_clip_top = 0;
    state.render_clip_bottom = SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM;
    state.active_dialogue_record = SHIP_3D_TARGET_EXIT_SENTINEL;
    state.closing = true;
    state.depth_step = SHIP_3D_NAVIGATION_TRIGGER_CLOSE_STEP;
    effect.cleared_trigger = true;
    effect.started_sequence = true;
    effect.set_scene_band = true;
    effect.restored_render_clip = true;
    effect.cleared_active_dialogue_record = true;
    effect.requested_closing = true;

    Some(effect)
}

pub fn draw_ship_3d_target_list(
    state: &mut Ship3dTargetHitState,
    layout: Ship3dTargetListLayout,
    label_offsets: &[u16],
    width_table: &[u16],
    activate: bool,
    alias_source_offset: Option<u16>,
) -> Option<Ship3dTargetDrawResult> {
    let inner_width = layout
        .width
        .wrapping_sub(SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING);
    let x_origin = layout.x.wrapping_add(SHIP_3D_TARGET_DRAW_X_INSET);
    let mut y = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
    let mut commands = Vec::new();

    for (row_index, label_offset) in label_offsets.iter().copied().enumerate() {
        if label_offset == 0 || label_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            break;
        }
        let measured_width = *width_table.get(row_index)?;
        commands.push(Ship3dTargetDrawCommand {
            row_index,
            string_segment: Ship3dTargetTextSegment::TargetList,
            string_offset: if Some(label_offset) == alias_source_offset {
                SHIP_3D_TARGET_ALIAS_LABEL_OFFSET
            } else {
                label_offset
            },
            x: target_list_draw_x(x_origin, inner_width, measured_width),
            y,
            color: next_target_list_draw_color(state, activate),
            measured_width,
            extra_entry: false,
        });
        y = y.wrapping_add(SHIP_3D_TARGET_LAYOUT_ROW_STEP);
    }

    if layout.has_extra_entry {
        let row_index = commands.len();
        let measured_width = *width_table.get(row_index)?;
        commands.push(Ship3dTargetDrawCommand {
            row_index,
            string_segment: Ship3dTargetTextSegment::GameData,
            string_offset: SHIP_3D_TARGET_EXTRA_LABEL_OFFSET,
            x: target_list_draw_x(x_origin, inner_width, measured_width),
            y,
            color: next_target_list_draw_color(state, activate),
            measured_width,
            extra_entry: true,
        });
    }

    Some(Ship3dTargetDrawResult {
        commands,
        final_hover_counter: state.hover_row,
    })
}

pub fn select_ship_3d_target_record(
    state: &mut Ship3dTargetSelectorState,
    primary_targets: &[u16],
    fallback_targets: &[u16],
    query_index_ax: u16,
    phase_gate_complete: bool,
) -> Option<Ship3dTargetSelection> {
    state.target_fallback = false;
    let mut targets = primary_targets;
    if primary_targets.first().copied() == Some(SHIP_3D_TARGET_EXIT_SENTINEL) {
        targets = fallback_targets;
        state.target_fallback = true;
    }
    let used_fallback_table = state.target_fallback;

    let mut ran_layout_prepass = false;
    if state.target_select_phase & 1 != 0 {
        ran_layout_prepass = true;
        state.target_animation_tick = 0;
        state.target_select_phase = state.target_select_phase.wrapping_add(1);
    }

    if state.target_select_phase & 2 != 0 {
        if !phase_gate_complete {
            return Some(Ship3dTargetSelection {
                ax: 0,
                used_fallback_table,
                ran_layout_prepass,
                phase_gate_blocked: true,
            });
        }
        state.target_select_phase = 0;
    }

    if query_index_ax == SHIP_3D_TARGET_EXIT_SENTINEL {
        return Some(Ship3dTargetSelection {
            ax: 0,
            used_fallback_table,
            ran_layout_prepass,
            phase_gate_blocked: false,
        });
    }

    let selected = targets.get(query_index_ax as usize).copied()?;
    if selected == SHIP_3D_TARGET_EXIT_SENTINEL {
        state.opening = true;
        state.depth_step = SHIP_3D_TARGET_OPEN_STEP;
        return Some(Ship3dTargetSelection {
            ax: SHIP_3D_TARGET_EXIT_SENTINEL,
            used_fallback_table,
            ran_layout_prepass,
            phase_gate_blocked: false,
        });
    }

    let ax = if state.target_fallback {
        state.current_target
    } else {
        selected.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES)
    };
    Some(Ship3dTargetSelection {
        ax,
        used_fallback_table,
        ran_layout_prepass,
        phase_gate_blocked: false,
    })
}

pub fn ship_3d_plane_band_byte_count(depth_offset: u16) -> usize {
    let rows = (depth_offset as u8).wrapping_add(SHIP_3D_PLANE_BASE_ROWS as u8) as usize;
    rows * SHIP_3D_PLANE_ROW_BYTES
}

pub fn ship_3d_scroll_value(depth_offset: u16) -> u16 {
    let doubled = depth_offset.wrapping_mul(2);
    let capped = if (doubled as i16) > 0x64 {
        0x64
    } else {
        doubled
    };
    0x64u16.wrapping_sub(capped)
}

fn start_closing_transition(state: &mut Ship3dTransitionState) {
    state.depth_step = SHIP_3D_TRANSITION_CLOSE_STEP;
    state.closing = true;
    state.transition_armed = false;
}

fn add_to_low_byte(value: u16, addend: u8) -> u16 {
    (value & 0xff00) | value.to_le_bytes()[0].wrapping_add(addend) as u16
}

fn circular_delta(first: u16, second: u16, modulus: u16) -> u16 {
    let (max, min) = if signed_i16(first) > signed_i16(second) {
        (first, second)
    } else {
        (second, first)
    };
    let delta = max.wrapping_sub(min);
    if signed_i16(delta) < signed_i16(modulus >> 1) {
        delta
    } else {
        modulus.wrapping_sub(delta)
    }
}

fn wrap_ring_once(value: i32, modulus: u16) -> u16 {
    if value < 0 {
        value.wrapping_add(modulus as i32) as u16
    } else if value >= modulus as i32 {
        value.wrapping_sub(modulus as i32) as u16
    } else {
        value as u16
    }
}

fn matrix_pair_for_angle(angle_table: &[Ship3dAngleTableEntry], angle: u16) -> Option<(i32, i32)> {
    let entry = *angle_table.get(usize::from(angle))?;
    Some((
        i32::from(entry.cosine).wrapping_mul(2),
        i32::from(entry.sine).wrapping_mul(2),
    ))
}

fn fixed_mul_shift_15(lhs: i32, rhs: i32) -> i32 {
    lhs.wrapping_mul(rhs) >> SHIP_3D_MATRIX_FIXED_SHIFT
}

fn projection_component(point_component: u16, origin_component: u16) -> i32 {
    i32::from(signed_i16(point_component.wrapping_sub(origin_component)))
}

fn projection_dot(components: [i32; 3], terms: [i32; 3]) -> i32 {
    components[0]
        .wrapping_mul(terms[0])
        .wrapping_add(components[1].wrapping_mul(terms[1]))
        .wrapping_add(components[2].wrapping_mul(terms[2]))
}

fn project_ship_3d_axis(numerator: i32, depth: i32, center: u16) -> u16 {
    let quotient = numerator / depth;
    (quotient as u16).wrapping_add(center)
}

fn scale_ship_3d_object_dimension(dimension: u16, depth_scale: u16) -> u16 {
    (u32::from(dimension).wrapping_mul(u32::from(depth_scale)) >> SHIP_3D_OBJECT_SCALE_SHIFT) as u16
}

fn ship_3d_rects_intersect(
    slot_rect: Ship3dProjectionViewport,
    dirty_rect: Ship3dProjectionViewport,
) -> bool {
    signed_i16(slot_rect.left) < signed_i16(dirty_rect.right)
        && signed_i16(slot_rect.top) < signed_i16(dirty_rect.bottom)
        && signed_i16(slot_rect.right) > signed_i16(dirty_rect.left)
        && signed_i16(slot_rect.bottom) > signed_i16(dirty_rect.top)
}

fn checked_i16_div_i8_to_i8(dividend: i16, divisor: i8) -> Option<i8> {
    if divisor == 0 {
        return None;
    }
    let quotient = dividend / divisor as i16;
    i8::try_from(quotient).ok()
}

fn checked_u16_div_u8_to_u8(dividend: u16, divisor: u8) -> Option<u8> {
    if divisor == 0 {
        return None;
    }
    let quotient = dividend / divisor as u16;
    u8::try_from(quotient).ok()
}

fn signed_i16(value: u16) -> i16 {
    value as i16
}

fn target_list_draw_x(x_origin: u16, inner_width: u16, measured_width: u16) -> u16 {
    x_origin.wrapping_add(inner_width.wrapping_sub(measured_width) >> 1)
}

impl Ship3dNavChoiceGates {
    fn blocks_nav_choice(self) -> bool {
        self.c2_presentation_gate
            || self.left_motion_gate
            || self.right_motion_gate
            || self.menu_gate
            || self.sound_gate
            || self.presentation_active
    }
}

fn hit_test_ship_3d_nav_choice(
    dynamic_axis: u16,
    mouse_x: u16,
    mouse_y: u16,
) -> Option<Option<u8>> {
    let relative_axis = dynamic_axis.wrapping_sub(SHIP_3D_NAV_CHOICE_AXIS_BIAS);
    let right = SHIP_3D_NAV_CHOICE_RIGHT_BASE.wrapping_sub(relative_axis.wrapping_shl(3));
    if signed_i16(mouse_x) > signed_i16(right) {
        return Some(None);
    }

    let left = right.wrapping_sub(SHIP_3D_NAV_CHOICE_X_WIDTH);
    if signed_i16(left) < 0 || signed_i16(mouse_x) < signed_i16(left) {
        return Some(None);
    }

    let abs_axis = if signed_i16(relative_axis) < 0 {
        0u16.wrapping_sub(relative_axis)
    } else {
        relative_axis
    };
    let quarter_axis = abs_axis >> 2;
    let y_origin = SHIP_3D_NAV_CHOICE_Y_BASE
        .wrapping_add(abs_axis)
        .wrapping_add(quarter_axis);
    let row_height = SHIP_3D_NAV_CHOICE_ROW_HEIGHT_BASE.wrapping_sub((quarter_axis as u8) >> 1);
    let row_offset = mouse_y.wrapping_sub(y_origin);
    if signed_i16(row_offset) < 0 {
        return Some(None);
    }

    let choice = checked_u16_div_u8_to_u8(row_offset, row_height)?;
    if choice >= SHIP_3D_NAV_CHOICE_COUNT {
        return Some(None);
    }
    Some(Some(choice))
}

fn adjust_nav_choice_target_records(target_records: &mut [u16]) {
    for target_record in target_records {
        if *target_record == SHIP_3D_TARGET_EXIT_SENTINEL {
            break;
        }
        *target_record = target_record.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES);
    }
}

fn rebuild_nav_choice_special_target_records(
    special_slots: &[u16],
    target_records: &mut Vec<u16>,
) -> Option<()> {
    target_records.clear();
    for special_slot in special_slots {
        if *special_slot == 0 {
            continue;
        }
        if *special_slot == SHIP_3D_TARGET_EXIT_SENTINEL {
            target_records.push(SHIP_3D_TARGET_EXIT_SENTINEL);
            return Some(());
        }
        target_records.push(special_slot.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
    }
    None
}

fn next_ship_3d_temp_snd_phase(phase: u8) -> u8 {
    let next = phase.wrapping_add(1);
    if next == SHIP_3D_TEMP_SND_PHASE_COUNT {
        0
    } else {
        next
    }
}

fn append_ship_3d_navigation_source_children(
    source_entries: &[Ship3dNavigationSourceEntry],
    records: &[Ship3dNavigationRuntimeRecord],
    parent_target: u16,
    source_records: &mut Vec<u16>,
) -> Option<()> {
    if source_entries.is_empty() {
        return None;
    }

    let mut index = 0;
    loop {
        let entry = source_entries.get(index)?;
        let record = find_ship_3d_navigation_record(records, entry.record_offset)?;
        if record.source_parent == Some(parent_target) {
            source_records.push(record.offset);
            append_ship_3d_navigation_source_children(
                source_entries,
                records,
                record.offset,
                source_records,
            )?;
        }

        index += 1;
        if source_entries.get(index).map(|entry| entry.entry_kind) != Some(1) {
            break;
        }
    }

    Some(())
}

fn find_ship_3d_navigation_record(
    records: &[Ship3dNavigationRuntimeRecord],
    offset: u16,
) -> Option<Ship3dNavigationRuntimeRecord> {
    records
        .iter()
        .copied()
        .find(|record| record.offset == offset)
}

fn find_ship_3d_position_record(
    records: &[Ship3dPositionRecord],
    offset: u16,
) -> Option<Ship3dPositionRecord> {
    records
        .iter()
        .copied()
        .find(|record| record.offset == offset)
}

fn find_ship_3d_position_field(
    fields: &[Ship3dPositionField],
    offset: u16,
) -> Option<Ship3dPositionField> {
    fields.iter().copied().find(|field| field.offset == offset)
}

fn resolve_ship_3d_distance_position_field(
    records: &[Ship3dPositionRecord],
    record: Ship3dPositionRecord,
    other_record: Ship3dPositionRecord,
    arche_object: u16,
    inherited_kind100_compare_word: u16,
) -> Option<u16> {
    if record.kind_flags == SHIP_3D_OBJECT_KIND_POSITION_KIND100 {
        return resolve_ship_3d_position_field(
            records,
            record.offset,
            arche_object,
            kind100_relation_word(other_record)?,
        );
    }

    resolve_ship_3d_position_field(
        records,
        record.offset,
        arche_object,
        inherited_kind100_compare_word,
    )
}

fn kind100_relation_word(record: Ship3dPositionRecord) -> Option<u16> {
    match vm::vm_field_offset(
        SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD,
        record.kind_flags,
    )? {
        0 => Some(record.kind_flags),
        _ => record.kind100_relation_word,
    }
}

fn ship_3d_record_field(record_offset: u16, kind_flags: u16, selector: u8) -> Option<u16> {
    vm::vm_field_offset(selector, kind_flags).map(|field| record_offset.wrapping_add(field))
}

fn binary_abs_word_diff(first: u16, second: u16) -> u16 {
    let diff = first.wrapping_sub(second);
    if diff & 0x8000 != 0 {
        diff.wrapping_neg()
    } else {
        diff
    }
}

fn next_target_list_draw_color(state: &mut Ship3dTargetHitState, activate: bool) -> u8 {
    state.hover_row = state.hover_row.wrapping_sub(1);
    if state.hover_row == 0 {
        if activate {
            SHIP_3D_TARGET_ACTIVE_TEXT_COLOR
        } else {
            SHIP_3D_TARGET_HOVER_TEXT_COLOR
        }
    } else {
        SHIP_3D_TARGET_DEFAULT_TEXT_COLOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_hud_pyramid_vertices_project_via_shared_projection() {
        // The recovered HUD geometry runs through the same projection as the ship
        // view / overlays. With the HUD entry angle (0xB3) at least some vertices
        // project to valid on-screen depths (>0), confirming the data + pipeline.
        assert_eq!(SHIP_3D_HUD_PYRAMID_VERTICES.len(), 32);
        let matrix = build_ship_3d_projection_matrix(
            &SHIP_3D_ANGLE_TABLE,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                projection_angle_2f6d: 0,
                angle_2f6f: 0,
            },
        )
        .expect("matrix");
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };
        let projected = SHIP_3D_HUD_PYRAMID_VERTICES
            .iter()
            .filter_map(|v| {
                project_ship_3d_point(
                    Ship3dProjectionPoint {
                        x: v[0] as u16,
                        y: v[1] as u16,
                        z: v[2] as u16,
                    },
                    origin,
                    matrix,
                )
            })
            .count();
        assert!(projected > 0, "some HUD vertices must project on-screen");
    }

    #[test]
    fn pyramid_hud_draws_only_in_the_bottom_band() {
        let mut fb = vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
        render_ship_3d_pyramid_hud(&mut fb, 0x80, 0xFD);
        // Draws grid + orb pixels...
        assert!(fb.iter().any(|&p| p == 0x80), "pyramid grid drawn");
        assert!(fb.iter().any(|&p| p == 0xFD), "eye-orb drawn");
        // ...and NOTHING above the HUD band (the scene band stays untouched).
        let band_start = SHIP_3D_HUD_BAND_TOP * SHIP_3D_PROJECTION_SCREEN_WIDTH;
        assert!(
            fb[..band_start].iter().all(|&p| p == 0),
            "HUD render must not touch the scene band above row 165"
        );
    }

    fn source_segment() -> Vec<u8> {
        (0..SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET + SHIP_3D_PLANE_PAGE_BYTES)
            .map(|idx| (idx & 0xff) as u8)
            .collect()
    }

    #[test]
    fn transition_state_starts_opening_after_hold_threshold() {
        let mut state = Ship3dTransitionState {
            hold_ticks: SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut state, true);
        assert_eq!(
            state,
            Ship3dTransitionState {
                hold_ticks: SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD,
                ..Ship3dTransitionState::default()
            }
        );

        state.hold_ticks = SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD + 1;
        update_ship_3d_transition_state(&mut state, false);
        assert_eq!(state.depth_step, SHIP_3D_TRANSITION_OPEN_STEP);
        assert!(state.opening);
        assert!(state.transition_armed);
        assert!(!state.closing);
    }

    #[test]
    fn transition_state_starts_closing_when_armed_timer_expires_or_random_gate_hits() {
        let mut expired = Ship3dTransitionState {
            transition_armed: true,
            hold_ticks: 0,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut expired, false);
        assert_eq!(expired.depth_step, SHIP_3D_TRANSITION_CLOSE_STEP);
        assert!(expired.closing);
        assert!(!expired.transition_armed);

        let mut gated = Ship3dTransitionState {
            transition_armed: true,
            hold_ticks: 1,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut gated, false);
        assert_eq!(
            gated,
            Ship3dTransitionState {
                transition_armed: true,
                hold_ticks: 1,
                ..Ship3dTransitionState::default()
            }
        );
        update_ship_3d_transition_state(&mut gated, true);
        assert_eq!(gated.depth_step, SHIP_3D_TRANSITION_CLOSE_STEP);
        assert!(gated.closing);
        assert!(!gated.transition_armed);
    }

    #[test]
    fn depth_scroll_opens_to_max_then_clears_opening_flag() {
        let mut state = Ship3dDepthState {
            depth_offset: 0x3c,
            opening: true,
            depth_step: SHIP_3D_TRANSITION_OPEN_STEP,
            ..Ship3dDepthState::default()
        };

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0x40);
        assert!(state.opening);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, SHIP_3D_MAX_DEPTH_OFFSET);
        assert!(state.opening);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, SHIP_3D_MAX_DEPTH_OFFSET);
        assert!(!state.opening);
    }

    #[test]
    fn depth_scroll_closes_to_zero_then_clears_closing_flag() {
        let mut state = Ship3dDepthState {
            depth_offset: 5,
            closing: true,
            depth_step: SHIP_3D_TRANSITION_CLOSE_STEP,
            ..Ship3dDepthState::default()
        };

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0);
        assert!(state.closing);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0);
        assert!(!state.closing);
    }

    #[test]
    fn depth_scroll_uses_8086_low_byte_arithmetic() {
        let mut closing = Ship3dDepthState {
            depth_offset: 0x0101,
            closing: true,
            depth_step: 1,
            ..Ship3dDepthState::default()
        };
        step_ship_3d_depth_scroll(&mut closing);
        assert_eq!(closing.depth_offset, 0x0100);

        let mut opening = Ship3dDepthState {
            depth_offset: 0xff00,
            opening: true,
            depth_step: 1,
            ..Ship3dDepthState::default()
        };
        step_ship_3d_depth_scroll(&mut opening);
        assert_eq!(opening.depth_offset, 0xff01);

        assert_eq!(
            ship_3d_plane_band_byte_count(0x0100),
            SHIP_3D_PLANE_BASE_ROWS * SHIP_3D_PLANE_ROW_BYTES
        );
    }

    #[test]
    fn plane_band_copy_uses_depth_plus_35_planar_rows() {
        let source = source_segment();
        let mut dest = vec![0xee; SHIP_3D_PLANE_DEST_BYTES];
        let copied =
            copy_ship_3d_plane_bands(&mut dest, &source, 0, true, 0).expect("ship 3D plane copy");

        assert_eq!(copied.row_count, SHIP_3D_PLANE_BASE_ROWS);
        assert_eq!(
            copied.byte_count,
            SHIP_3D_PLANE_BASE_ROWS * SHIP_3D_PLANE_ROW_BYTES
        );
        assert_eq!(
            copied.first_source_start,
            SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + SHIP_3D_PLANE_PAGE_BYTES - copied.byte_count
        );
        assert_eq!(
            &dest[0..copied.byte_count],
            &source[copied.first_source_start..copied.first_source_start + copied.byte_count]
        );
        assert_eq!(
            &dest[copied.second_dest_start..copied.second_dest_start + copied.byte_count],
            &source[copied.second_source_start..copied.second_source_start + copied.byte_count]
        );
        assert!(
            dest[copied.byte_count..copied.second_dest_start]
                .iter()
                .all(|value| *value == 0xee)
        );
        assert_eq!(copied.new_scroll_value, Some(0x64));
    }

    #[test]
    fn plane_band_copy_at_max_depth_copies_two_full_planar_pages() {
        let source = source_segment();
        let mut dest = vec![0; SHIP_3D_PLANE_DEST_BYTES];
        let copied = copy_ship_3d_plane_bands(
            &mut dest,
            &source,
            SHIP_3D_MAX_DEPTH_OFFSET,
            true,
            SHIP_3D_SCROLL_MODE_HOLD,
        )
        .expect("ship 3D plane copy");

        assert_eq!(copied.row_count, 100);
        assert_eq!(copied.byte_count, SHIP_3D_PLANE_PAGE_BYTES);
        assert_eq!(copied.first_source_start, SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET);
        assert_eq!(copied.second_dest_start, SHIP_3D_PLANE_PAGE_BYTES);
        assert_eq!(
            &dest[0..SHIP_3D_PLANE_PAGE_BYTES],
            &source[SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET
                ..SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + SHIP_3D_PLANE_PAGE_BYTES]
        );
        assert_eq!(
            &dest[SHIP_3D_PLANE_PAGE_BYTES..SHIP_3D_PLANE_DEST_BYTES],
            &source[SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET
                ..SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET + SHIP_3D_PLANE_PAGE_BYTES]
        );
        assert_eq!(copied.new_scroll_value, None);
    }

    #[test]
    fn plane_band_copy_reports_scroll_value_like_binary_math() {
        assert_eq!(ship_3d_scroll_value(0), 0x64);
        assert_eq!(ship_3d_scroll_value(30), 40);
        assert_eq!(ship_3d_scroll_value(50), 0);
        assert_eq!(ship_3d_scroll_value(SHIP_3D_MAX_DEPTH_OFFSET), 0);
        assert_eq!(copy_ship_3d_plane_bands(&mut [], &[], 0, false, 0), None);
    }

    #[test]
    fn target_selector_runs_phase_prepass_and_blocks_while_gate_is_active() {
        let mut state = Ship3dTargetSelectorState {
            target_select_phase: 1,
            target_animation_tick: 7,
            ..Ship3dTargetSelectorState::default()
        };

        let selected = select_ship_3d_target_record(&mut state, &[0x1200], &[], 0, false).unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                ax: 0,
                used_fallback_table: false,
                ran_layout_prepass: true,
                phase_gate_blocked: true,
            }
        );
        assert_eq!(state.target_select_phase, 2);
        assert_eq!(state.target_animation_tick, 0);
    }

    #[test]
    fn target_selector_returns_primary_target_after_phase_gate_completes() {
        let mut state = Ship3dTargetSelectorState {
            target_select_phase: 2,
            ..Ship3dTargetSelectorState::default()
        };

        let selected =
            select_ship_3d_target_record(&mut state, &[0x1200, 0x2345], &[], 1, true).unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                ax: 0x2341,
                used_fallback_table: false,
                ran_layout_prepass: false,
                phase_gate_blocked: false,
            }
        );
        assert_eq!(state.target_select_phase, 0);
        assert!(!state.opening);
    }

    #[test]
    fn target_selector_fallback_table_returns_current_target() {
        let mut state = Ship3dTargetSelectorState {
            current_target: 0x4567,
            ..Ship3dTargetSelectorState::default()
        };

        let selected = select_ship_3d_target_record(
            &mut state,
            &[SHIP_3D_TARGET_EXIT_SENTINEL],
            &[0x2222],
            0,
            true,
        )
        .unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                ax: 0x4567,
                used_fallback_table: true,
                ran_layout_prepass: false,
                phase_gate_blocked: false,
            }
        );
        assert!(state.target_fallback);
    }

    #[test]
    fn target_selector_exit_sentinel_arms_opening_transition() {
        let mut state = Ship3dTargetSelectorState::default();

        let selected = select_ship_3d_target_record(
            &mut state,
            &[0x1200, SHIP_3D_TARGET_EXIT_SENTINEL],
            &[],
            1,
            true,
        )
        .unwrap();

        assert_eq!(selected.ax, SHIP_3D_TARGET_EXIT_SENTINEL);
        assert!(state.opening);
        assert_eq!(state.depth_step, SHIP_3D_TARGET_OPEN_STEP);
    }

    #[test]
    fn target_selector_no_query_selection_returns_zero_ax() {
        let mut state = Ship3dTargetSelectorState::default();

        let selected = select_ship_3d_target_record(
            &mut state,
            &[0x1200],
            &[],
            SHIP_3D_TARGET_EXIT_SENTINEL,
            true,
        )
        .unwrap();

        assert_eq!(selected.ax, 0);
        assert!(!state.opening);
    }

    #[test]
    fn interpolation_gate_reports_complete_without_advancing_at_duration() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 6,
        };

        let step = step_ship_3d_interpolation_gate(&mut gate, [10, 20, 30, 40], [0, 0, 0, 0]);

        assert_eq!(step, Some(Ship3dInterpolationStep::Complete));
        assert_eq!(gate.current_tick, 6);
    }

    #[test]
    fn interpolation_gate_increments_tick_and_interpolates_four_words() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 1,
        };

        let step = step_ship_3d_interpolation_gate(&mut gate, [60, 66, 72, 78], [0, 6, 12, 18]);

        assert_eq!(
            step,
            Some(Ship3dInterpolationStep::Active([20, 26, 32, 38]))
        );
        assert_eq!(gate.current_tick, 2);
    }

    #[test]
    fn interpolation_gate_uses_signed_truncating_division() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 2,
        };

        let step = step_ship_3d_interpolation_gate(
            &mut gate,
            [0xfff0, 0x0000, 0x0031, 0x0000],
            [0, 31, 0, 0],
        );

        assert_eq!(
            step,
            Some(Ship3dInterpolationStep::Active([
                0xfffa, // (-16 / 6) * 3 = -6, added to 0.
                0x0010, // (-31 / 6) * 3 = -15, added to 31.
                0x0018, // (49 / 6) * 3 = 24.
                0,
            ]))
        );
        assert_eq!(gate.current_tick, 3);
    }

    #[test]
    fn interpolation_gate_rejects_binary_idiv_error_shapes() {
        let mut zero_duration = Ship3dInterpolationGate {
            duration_ticks: 0,
            current_tick: 1,
        };
        assert_eq!(
            step_ship_3d_interpolation_gate(&mut zero_duration, [1, 0, 0, 0], [0, 0, 0, 0]),
            None
        );

        let mut quotient_overflow = Ship3dInterpolationGate {
            duration_ticks: 1,
            current_tick: 0,
        };
        assert_eq!(
            step_ship_3d_interpolation_gate(
                &mut quotient_overflow,
                [0x0100, 0, 0, 0],
                [0, 0, 0, 0]
            ),
            None
        );
    }

    #[test]
    fn target_list_layout_uses_binary_default_width_floor_and_centering() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);

        assert_eq!(
            layout,
            Ship3dTargetListLayout {
                x: 20,
                y: 85,
                width: 120,
                height: 30,
                max_label_width: SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH,
                label_count: 2,
                has_extra_entry: false,
                selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            }
        );
    }

    #[test]
    fn target_list_layout_grows_to_widest_label() {
        let layout = layout_ship_3d_target_list(&[120, 50], 0x50, false);

        assert_eq!(layout.max_label_width, 120);
        assert_eq!(layout.width, 140);
        assert_eq!(layout.x, 10);
        assert_eq!(layout.height, 30);
        assert_eq!(layout.y, 85);
    }

    #[test]
    fn target_list_layout_extra_entry_uses_shorter_width_and_height_seed() {
        let layout = layout_ship_3d_target_list(&[], 0x50, true);

        assert_eq!(
            layout,
            Ship3dTargetListLayout {
                x: 43,
                y: 91,
                width: 75,
                height: 18,
                max_label_width: SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH,
                label_count: 0,
                has_extra_entry: true,
                selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            }
        );
    }

    #[test]
    fn target_list_layout_preserves_binary_wrapping_for_tall_lists() {
        let widths = vec![1; 20];
        let layout = layout_ship_3d_target_list(&widths, 0x50, false);

        assert_eq!(layout.height, 228);
        assert_eq!(layout.y, 0x7ff2);
    }

    #[test]
    fn target_hit_test_commits_selection_only_when_active() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        let hover = hit_test_ship_3d_target_list(&mut state, layout, 30, 90, false).unwrap();
        assert_eq!(
            hover,
            Ship3dTargetHitResult {
                inside: true,
                activated: false,
                hover_row: 1,
                selected_row: 0,
                return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
                play_select_sound: false,
            }
        );
        assert_eq!(state.hover_row, 1);
        assert_eq!(state.selected_row, 0);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_HOVER_PRESENTATION_MODE
        );

        let active = hit_test_ship_3d_target_list(&mut state, layout, 30, 101, true).unwrap();
        assert_eq!(
            active,
            Ship3dTargetHitResult {
                inside: true,
                activated: true,
                hover_row: 2,
                selected_row: 2,
                return_ax: 1,
                play_select_sound: true,
            }
        );
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE
        );
    }

    #[test]
    fn target_hit_test_uses_inclusive_x_and_exclusive_bottom_y() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        assert!(
            hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x,
                layout.y + SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x + layout.width,
                layout.y + layout.height - SHIP_3D_TARGET_HIT_TEST_TOP_INSET - 1,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            !hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x + layout.width + 1,
                layout.y + SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            !hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x,
                layout.y + layout.height - SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
    }

    #[test]
    fn target_hit_test_clears_selection_then_requests_idle_when_outside() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState {
            selected_row: 2,
            presentation_state: SHIP_3D_TARGET_HOVER_PRESENTATION_MODE,
            requested_presentation_state: SHIP_3D_TARGET_HOVER_PRESENTATION_MODE,
            ..Ship3dTargetHitState::default()
        };

        let result = hit_test_ship_3d_target_list(&mut state, layout, 0, 0, false).unwrap();

        assert_eq!(
            result,
            Ship3dTargetHitResult {
                inside: false,
                activated: false,
                hover_row: 0,
                selected_row: 0,
                return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
                play_select_sound: false,
            }
        );
        assert_eq!(state.hover_row, 0);
        assert_eq!(state.selected_row, 0);
        assert_eq!(state.presentation_state, 0);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_IDLE_PRESENTATION_MODE
        );
    }

    #[test]
    fn target_hit_test_rejects_binary_div_overflow_shape() {
        let layout = Ship3dTargetListLayout {
            x: 0,
            y: 0,
            width: 1,
            height: 0x0b09,
            max_label_width: 1,
            label_count: 256,
            has_extra_entry: false,
            selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        };
        let mut state = Ship3dTargetHitState::default();

        assert_eq!(
            hit_test_ship_3d_target_list(&mut state, layout, 0, 0x0b04, false),
            None
        );
    }

    #[test]
    fn target_draw_centers_rows_from_width_table_and_highlights_hover() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState {
            hover_row: 2,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, 0x2000, SHIP_3D_TARGET_EXIT_SENTINEL],
            &[20, 80],
            false,
            None,
        )
        .unwrap();

        assert_eq!(
            drawn.commands,
            vec![
                Ship3dTargetDrawCommand {
                    row_index: 0,
                    string_segment: Ship3dTargetTextSegment::TargetList,
                    string_offset: 0x1000,
                    x: 70,
                    y: 89,
                    color: SHIP_3D_TARGET_DEFAULT_TEXT_COLOR,
                    measured_width: 20,
                    extra_entry: false,
                },
                Ship3dTargetDrawCommand {
                    row_index: 1,
                    string_segment: Ship3dTargetTextSegment::TargetList,
                    string_offset: 0x2000,
                    x: 40,
                    y: 100,
                    color: SHIP_3D_TARGET_HOVER_TEXT_COLOR,
                    measured_width: 80,
                    extra_entry: false,
                },
            ]
        );
        assert_eq!(drawn.final_hover_counter, 0);
        assert_eq!(state.hover_row, 0);
    }

    #[test]
    fn target_draw_uses_active_color_and_keeps_decrementing_after_hover() {
        let layout = layout_ship_3d_target_list(&[20, 80, 40], 0x50, false);
        let mut state = Ship3dTargetHitState {
            hover_row: 1,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, 0x2000, 0x3000],
            &[20, 80, 40],
            true,
            None,
        )
        .unwrap();

        assert_eq!(drawn.commands[0].color, SHIP_3D_TARGET_ACTIVE_TEXT_COLOR);
        assert_eq!(drawn.commands[1].color, SHIP_3D_TARGET_DEFAULT_TEXT_COLOR);
        assert_eq!(drawn.commands[2].color, SHIP_3D_TARGET_DEFAULT_TEXT_COLOR);
        assert_eq!(drawn.final_hover_counter, 0xfe);
    }

    #[test]
    fn target_draw_stops_at_sentinel_then_draws_cancel_extra_entry() {
        let layout = layout_ship_3d_target_list(&[20], 0x50, true);
        let mut state = Ship3dTargetHitState {
            hover_row: 2,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000],
            &[20, SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH],
            false,
            None,
        )
        .unwrap();

        assert_eq!(drawn.commands.len(), 2);
        assert_eq!(drawn.commands[0].string_offset, 0x1000);
        assert_eq!(drawn.commands[1].row_index, 1);
        assert_eq!(
            drawn.commands[1].string_segment,
            Ship3dTargetTextSegment::GameData
        );
        assert_eq!(
            drawn.commands[1].string_offset,
            SHIP_3D_TARGET_EXTRA_LABEL_OFFSET
        );
        assert_eq!(drawn.commands[1].color, SHIP_3D_TARGET_HOVER_TEXT_COLOR);
        assert!(drawn.commands[1].extra_entry);
    }

    #[test]
    fn target_draw_applies_alias_blank_label_offset() {
        let layout = layout_ship_3d_target_list(&[20], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        let drawn =
            draw_ship_3d_target_list(&mut state, layout, &[0x4444], &[20], false, Some(0x4444))
                .unwrap();

        assert_eq!(
            drawn.commands[0].string_offset,
            SHIP_3D_TARGET_ALIAS_LABEL_OFFSET
        );
    }

    #[test]
    fn target_draw_requires_matching_width_table_entries() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        assert_eq!(
            draw_ship_3d_target_list(&mut state, layout, &[0x1000, 0x2000], &[20], false, None),
            None
        );
    }

    #[test]
    fn nav_choice_hover_maps_mouse_to_palette_highlight() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 2,
                activate: false,
            },
        )
        .unwrap();

        assert_eq!(
            result,
            Ship3dNavChoiceResult {
                gated: false,
                reset_palette_range: true,
                hovered_choice: Some(3),
                highlighted_palette_index: Some(SHIP_3D_NAV_CHOICE_PALETTE_FIRST + 2),
                committed_choice: None,
                dispatched_choice: None,
                play_select_sound: None,
            }
        );
        assert_eq!(state, Ship3dNavChoiceState::default());
    }

    #[test]
    fn nav_choice_activation_sets_binary_state_without_dispatching_yet() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MAX_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 3,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(result.hovered_choice, Some(4));
        assert_eq!(result.committed_choice, Some(4));
        assert_eq!(result.dispatched_choice, None);
        assert_eq!(
            result.play_select_sound,
            Some(SHIP_3D_NAV_CHOICE_SELECT_SOUND)
        );
        assert_eq!(state.selected_choice, 4);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_NAV_CHOICE_PRESENTATION_MODE
        );
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS);
        assert_eq!(state.hold_ticks, SHIP_3D_NAV_CHOICE_HOLD_TICKS);
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        assert_eq!(
            state.target_y,
            SHIP_3D_NAV_CHOICE_TARGET_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 3
        );
        assert!(state.target_layout_preserve_widths);
        assert_eq!(
            state.target_layout_center_x,
            SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X
        );
        assert!(state.target_layout_extra_entry);
        assert_eq!(
            state.interpolation_duration_ticks,
            SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION
        );
    }

    #[test]
    fn nav_choice_existing_selection_dispatches_after_hud_bit_clears() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: 0,
            ..Ship3dNavChoiceState::default()
        };

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: 0,
                dynamic_axis: 0,
                mouse_x: 0,
                mouse_y: 0,
                activate: false,
            },
        )
        .unwrap();

        assert_eq!(result.reset_palette_range, false);
        assert_eq!(result.hovered_choice, None);
        assert_eq!(result.dispatched_choice, Some(2));
    }

    #[test]
    fn nav_choice_gates_block_hit_test_and_dispatch() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            ..Ship3dNavChoiceState::default()
        };

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates {
                presentation_active: true,
                ..Ship3dNavChoiceGates::default()
            },
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(
            result,
            Ship3dNavChoiceResult {
                gated: true,
                ..Ship3dNavChoiceResult::default()
            }
        );
        assert_eq!(state.selected_choice, 2);
    }

    #[test]
    fn nav_choice_rejects_out_of_range_gate_before_palette_reset() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE - 1,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(result, Ship3dNavChoiceResult::default());
        assert_eq!(state.selected_choice, 0);
    }

    #[test]
    fn nav_choice_uses_dynamic_axis_for_slanted_bounds() {
        let mut state = Ship3dNavChoiceState::default();

        let outside = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS + 4,
                mouse_x: 0x0090,
                mouse_y: 0x004d,
                activate: false,
            },
        )
        .unwrap();
        assert_eq!(outside.reset_palette_range, true);
        assert_eq!(outside.hovered_choice, None);

        let inside = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS + 4,
                mouse_x: 0x0091,
                mouse_y: 0x004d,
                activate: false,
            },
        )
        .unwrap();
        assert_eq!(inside.hovered_choice, Some(1));
    }

    #[test]
    fn nav_choice_handler_0_defers_honk_record_link_and_clears_phase() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_duration_ticks: SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION,
            interpolation_current_tick: 3,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_0(&mut state, 0x6754);

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x6754),
                cleared_handler_phase: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
    }

    #[test]
    fn nav_choice_handler_0_returns_without_phase_bit() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: 0x02,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_0(&mut state, 0x6754);

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.handler_phase, 0x02);
    }

    #[test]
    fn nav_choice_handler_1_adjusts_records_and_waits_for_interpolation() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 3,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1000, 0x2000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000];

        let effect = run_ship_3d_nav_choice_handler_1(
            &mut state,
            &mut target_records,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(
            target_records,
            [0x1004, 0x2004, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000]
        );
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                adjusted_target_records: true,
                phase_gate_blocked: true,
                reset_interpolation_tick: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_1_selects_target_after_interpolation() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, 0x2004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_1(&mut state, &mut target_records, true, 1).unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x2000),
                cleared_handler_phase: true,
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_1_exit_sentinel_clears_choice_without_deferred_record() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_1(&mut state, &mut target_records, true, 1).unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_1_no_selection_leaves_state_armed() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect = run_ship_3d_nav_choice_handler_1(
            &mut state,
            &mut target_records,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.selected_choice, 2);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
    }

    #[test]
    fn nav_choice_handler_2_rebuilds_targets_from_special_slots_and_waits() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 7,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0xaaaa, 0xbbbb];

        let effect = run_ship_3d_nav_choice_handler_2(
            &mut state,
            &[0, 0x1200, 0, 0x3400, SHIP_3D_TARGET_EXIT_SENTINEL, 0x5600],
            &mut target_records,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(
            target_records,
            vec![0x1204, 0x3404, SHIP_3D_TARGET_EXIT_SENTINEL]
        );
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                rebuilt_target_records: true,
                reset_interpolation_tick: true,
                phase_gate_blocked: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_2_selects_special_slot_target_and_sets_input_gate() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 3,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0x1204, 0x3404, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_2(&mut state, &[], &mut target_records, true, 1)
                .unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_related: Some(0x3400),
                cleared_handler_phase: true,
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                set_input_gate_b: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_2_exit_sentinel_clears_choice_without_gate() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 3,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0x1204, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_2(&mut state, &[], &mut target_records, true, 1)
                .unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_2_requires_special_slot_sentinel_when_rebuilding() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0xaaaa];

        assert_eq!(
            run_ship_3d_nav_choice_handler_2(
                &mut state,
                &[0, 0x1200],
                &mut target_records,
                false,
                SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            ),
            None
        );
        assert_eq!(target_records, vec![0x1204]);
    }

    #[test]
    fn nav_choice_handler_3_defers_static_record_link_and_reloads_radio_bank() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 4,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 8,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_3(&mut state, 0x6756);

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x6756),
                cleared_handler_phase: true,
                load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 4);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
        assert_eq!(state.interpolation_current_tick, 8);
    }

    #[test]
    fn nav_choice_handler_3_returns_without_phase_bit() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_3(&mut state, 0x6756);

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
    }

    #[test]
    fn nav_choice_handler_4_runs_layout_snapshot_and_waits_for_interpolation() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 9,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State::default();
        let layout_rect = [0x10, 0x20, 0x30, 0x40];

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            layout_rect,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(handler_state.layout_rect_snapshot, layout_rect);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                copied_layout_rect_snapshot: true,
                reset_interpolation_tick: true,
                phase_gate_blocked: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_4_no_selection_leaves_choice_armed_after_phase_clear() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State::default();

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_handler_phase: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 5);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
    }

    #[test]
    fn nav_choice_handler_4_menu_choice_sets_both_menu_gates_and_clears_choice() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State::default();

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            0,
        );

        assert!(handler_state.menu_gate);
        assert!(handler_state.secondary_menu_gate);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_4_voc_choice_toggles_tablo2_playback() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State {
            voc_enabled: true,
            tablo2_voc_reset_gate: true,
            ..Ship3dNavChoiceHandler4State::default()
        };

        let start_effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            1,
        );

        assert_eq!(handler_state.voc_stream_phase, 0);
        assert!(handler_state.tablo2_voc_active);
        assert!(!handler_state.tablo2_voc_reset_gate);
        assert_eq!(
            handler_state.active_target_list_offset,
            SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_ON_TARGET_LIST_OFFSET
        );
        assert_eq!(
            start_effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                load_voc_path: Some(SHIP_3D_NAV_CHOICE_TABLO2_VOC_PATH_OFFSET),
                start_voc_playback: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );

        state.selected_choice = 5;
        state.hud_flags = SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
        let stop_effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            1,
        );

        assert_eq!(handler_state.voc_stream_phase, 0);
        assert!(!handler_state.tablo2_voc_active);
        assert_eq!(
            handler_state.active_target_list_offset,
            SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_OFF_TARGET_LIST_OFFSET
        );
        assert_eq!(
            stop_effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_4_motion_choices_set_left_and_right_gates() {
        let mut left_state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut left_handler_state = Ship3dNavChoiceHandler4State::default();

        let left_effect = run_ship_3d_nav_choice_handler_4(
            &mut left_state,
            &mut left_handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            2,
        );

        assert!(left_handler_state.shared_motion_gate);
        assert!(left_handler_state.left_motion_gate);
        assert!(!left_handler_state.right_motion_gate);
        assert!(left_effect.cleared_selected_choice);
        assert!(left_effect.cleared_hud_target_list_flag);

        let mut right_state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut right_handler_state = Ship3dNavChoiceHandler4State::default();

        let right_effect = run_ship_3d_nav_choice_handler_4(
            &mut right_state,
            &mut right_handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            3,
        );

        assert!(right_handler_state.shared_motion_gate);
        assert!(!right_handler_state.left_motion_gate);
        assert!(right_handler_state.right_motion_gate);
        assert!(right_effect.cleared_selected_choice);
        assert!(right_effect.cleared_hud_target_list_flag);
    }

    #[test]
    fn nav_choice_handler_4_sound_choice_blocks_dispatch_and_clears_activation() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State {
            target_activate_flag: true,
            target_activate_secondary_flag: true,
            ..Ship3dNavChoiceHandler4State::default()
        };

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            4,
        );

        assert_eq!(
            handler_state.sound_gate,
            SHIP_3D_NAV_CHOICE_SOUND_GATE_SUPPRESS_TARGETS
        );
        assert!(!handler_state.target_activate_flag);
        assert!(!handler_state.target_activate_secondary_flag);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn procedural_update_rotates_active_hud_toward_hold_angle() {
        let mut state = Ship3dProceduralUpdateState {
            hud_flags: SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG,
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING,
            mouse_y: 0x0064,
            hold_ticks: 100,
            nav_timer: 0,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                initialized_nav_timer: true,
                applied_hud_rotation: true,
                updated_projection_angle: true,
                mouse_set_position: Some((0x0640, 0x0064)),
                carry_set: true,
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 30);
        assert_eq!(state.nav_timer, 40);
        assert_eq!(state.mouse_delta_accumulator, 160);
        assert_eq!(state.mouse_button_state, 0);
        assert!(state.rotation_direction_positive);
        assert_eq!(state.projection_angle, 30);
        assert_eq!(state.rotation_offset, 80);
        assert_eq!(state.mouse_x, 80);
        assert_eq!(state.mouse_sector, 40);
    }

    #[test]
    fn procedural_update_auto_rotates_angle_when_hud_inactive() {
        let mut state = Ship3dProceduralUpdateState {
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING + 0x01e0,
            mouse_y: 0x0070,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                auto_rotated_angle: true,
                updated_projection_angle: true,
                mouse_set_position: Some((0x0780, 0x0070)),
                carry_set: true,
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 45);
        assert_eq!(state.projection_angle, 45);
        assert_eq!(state.rotation_offset, 200);
        assert_eq!(state.mouse_x, 280);
        assert_eq!(state.mouse_sector, 120);
        assert!(state.rotation_direction_positive);
    }

    #[test]
    fn procedural_update_target_list_flag_adjusts_mouse_without_rotating_angle() {
        let mut state = Ship3dProceduralUpdateState {
            hud_flags: SHIP_3D_PROCEDURAL_TARGET_LIST_FLAG,
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING + 0x01e0,
            mouse_y: 0x0080,
            projection_angle: 77,
            rotation_offset: 0x0020,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                adjusted_target_list_mouse: true,
                mouse_set_position: Some((0x0690, 0x0080)),
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 10);
        assert_eq!(state.projection_angle, 77);
        assert_eq!(state.rotation_offset, 0x0020);
        assert_eq!(state.mouse_x, 208);
        assert_eq!(state.mouse_sector, 120);
    }

    #[test]
    fn procedural_update_close_angle_only_applies_existing_rotation_offset() {
        let mut state = Ship3dProceduralUpdateState {
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING + 0x0078,
            mouse_y: 0x0090,
            projection_angle: 66,
            rotation_offset: 0x0010,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                mouse_set_position: Some((0x0618, 0x0090)),
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 10);
        assert_eq!(state.projection_angle, 66);
        assert_eq!(state.rotation_offset, 0x0010);
        assert_eq!(state.mouse_x, 104);
        assert_eq!(state.mouse_sector, 30);
    }

    #[test]
    fn projection_matrix_builds_basis_orientation() {
        let angle_table = [
            Ship3dAngleTableEntry {
                cosine: 0x4000,
                sine: 0,
            },
            Ship3dAngleTableEntry {
                cosine: 0,
                sine: 0x4000,
            },
        ];

        let matrix = build_ship_3d_projection_matrix(
            &angle_table,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                projection_angle_2f6d: 0,
                angle_2f6f: 0,
            },
        )
        .unwrap();

        assert_eq!(
            matrix,
            Ship3dProjectionMatrix {
                terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000]
            }
        );
    }

    #[test]
    fn projection_matrix_preserves_binary_fixed_point_operation_order() {
        let angle_table = [
            Ship3dAngleTableEntry {
                cosine: 0x4000,
                sine: 0,
            },
            Ship3dAngleTableEntry {
                cosine: 0,
                sine: 0x4000,
            },
            Ship3dAngleTableEntry {
                cosine: 0x2000,
                sine: 0x2000,
            },
        ];

        let matrix = build_ship_3d_projection_matrix(
            &angle_table,
            Ship3dMatrixAngles {
                angle_2f71: 1,
                projection_angle_2f6d: 2,
                angle_2f6f: 0,
            },
        )
        .unwrap();

        assert_eq!(
            matrix.terms,
            [0, -32768, 0, -16384, 0, 16384, 16384, 0, 16384]
        );
    }

    #[test]
    fn projection_matrix_rejects_missing_angle_table_entry() {
        let angle_table = [Ship3dAngleTableEntry {
            cosine: 0x4000,
            sine: 0,
        }];

        assert_eq!(
            build_ship_3d_projection_matrix(
                &angle_table,
                Ship3dMatrixAngles {
                    angle_2f71: 0,
                    projection_angle_2f6d: 1,
                    angle_2f6f: 0,
                },
            ),
            None
        );
    }

    #[test]
    fn projection_point_uses_matrix_depth_and_screen_centers() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };

        let projected = project_ship_3d_point(
            Ship3dProjectionPoint {
                x: 10,
                y: 20,
                z: 1000,
            },
            Ship3dProjectionOrigin::default(),
            matrix,
        )
        .unwrap();

        assert_eq!(
            projected,
            Ship3dProjectedPoint {
                x: 162,
                y: 95,
                depth: 1000,
            }
        );
    }

    #[test]
    fn projection_point_rejects_zero_and_negative_depth_like_binary_branch() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };

        assert_eq!(
            project_ship_3d_point(
                Ship3dProjectionPoint { x: 0, y: 0, z: 0 },
                Ship3dProjectionOrigin::default(),
                matrix,
            ),
            None
        );
        assert_eq!(
            project_ship_3d_point(
                Ship3dProjectionPoint {
                    x: 0,
                    y: 0,
                    z: 0xffff,
                },
                Ship3dProjectionOrigin::default(),
                matrix,
            ),
            None
        );
    }

    #[test]
    fn projection_point_subtracts_origin_as_wrapping_words_before_sign_extend() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };

        let projected = project_ship_3d_point(
            Ship3dProjectionPoint {
                x: 0,
                y: 0,
                z: 1000,
            },
            Ship3dProjectionOrigin {
                x: 0xfc18,
                y: 0,
                z: 0,
            },
            matrix,
        )
        .unwrap();

        assert_eq!(projected.x, 416);
        assert_eq!(projected.y, 100);
        assert_eq!(projected.depth, 1000);
    }

    #[test]
    fn projection_plot_clips_occupied_pixels_and_writes_depth_shade() {
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: 320,
            top: 0,
            bottom: 200,
        };
        let projected = Ship3dProjectedPoint {
            x: 10,
            y: 2,
            depth: 0x3000,
        };
        let mut depth_buffer = vec![0; SHIP_3D_PROJECTION_SCREEN_WIDTH * 200];

        let pixel = plot_ship_3d_projected_point(&mut depth_buffer, viewport, projected).unwrap();

        assert_eq!(
            pixel,
            Ship3dProjectedPixel {
                offset: 650,
                shade: 0xec,
            }
        );
        assert_eq!(depth_buffer[650], 0xec);

        assert_eq!(
            plot_ship_3d_projected_point(&mut depth_buffer, viewport, projected),
            None
        );
        assert_eq!(depth_buffer[650], 0xec);

        assert_eq!(
            plot_ship_3d_projected_point(
                &mut depth_buffer,
                viewport,
                Ship3dProjectedPoint {
                    x: 320,
                    y: 2,
                    depth: 0x3000,
                },
            ),
            None
        );
    }

    #[test]
    fn object_sprite_projection_scales_and_centers_visible_descriptor() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG | SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG,
            source_width: 64,
            source_height: 32,
            draw_x: 0,
            draw_y: 0,
            extent_width: 64,
            extent_height: 32,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        let projection = project_ship_3d_object_sprite(
            Ship3dProjectionPoint {
                x: 10,
                y: 20,
                z: 1000,
            },
            Ship3dProjectionOrigin::default(),
            matrix,
            &mut descriptor,
        )
        .unwrap();

        assert_eq!(
            projection,
            Ship3dObjectSpriteProjection {
                projected: Ship3dProjectedPoint {
                    x: 162,
                    y: 95,
                    depth: 1000,
                },
                depth_scale: 1048,
                scaled_width: 65,
                scaled_height: 32,
                draw_x: 130,
                draw_y: 79,
            }
        );
        assert_eq!(
            descriptor,
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_OBJECT_VISIBLE_FLAG
                    | SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
                    | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                    | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG,
                source_width: 64,
                source_height: 32,
                draw_x: 130,
                draw_y: 79,
                extent_width: 65,
                extent_height: 32,
                ..Ship3dObjectSpriteDescriptor::default()
            }
        );
    }

    #[test]
    fn object_sprite_projection_skips_hidden_descriptor_and_zero_depth() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: 0,
            source_width: 64,
            source_height: 32,
            draw_x: 0,
            draw_y: 0,
            extent_width: 64,
            extent_height: 32,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            project_ship_3d_object_sprite(
                Ship3dProjectionPoint {
                    x: 10,
                    y: 20,
                    z: 1000,
                },
                Ship3dProjectionOrigin::default(),
                matrix,
                &mut descriptor,
            ),
            None
        );
        descriptor.flags = SHIP_3D_OBJECT_VISIBLE_FLAG;
        assert_eq!(
            project_ship_3d_object_sprite(
                Ship3dProjectionPoint { x: 0, y: 0, z: 0 },
                Ship3dProjectionOrigin::default(),
                matrix,
                &mut descriptor,
            ),
            None
        );
    }

    #[test]
    fn object_sprite_projection_wraps_negative_depth_before_scaling() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG,
            source_width: 64,
            source_height: 32,
            draw_x: 0,
            draw_y: 0,
            extent_width: 64,
            extent_height: 32,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        let projection = project_ship_3d_object_sprite(
            Ship3dProjectionPoint {
                x: 0,
                y: 0,
                z: 0xffff,
            },
            Ship3dProjectionOrigin::default(),
            matrix,
            &mut descriptor,
        )
        .unwrap();

        assert_eq!(
            projection,
            Ship3dObjectSpriteProjection {
                projected: Ship3dProjectedPoint {
                    x: 160,
                    y: 100,
                    depth: 0xffff,
                },
                depth_scale: 16,
                scaled_width: 1,
                scaled_height: 0,
                draw_x: 160,
                draw_y: 100,
            }
        );
        assert_eq!(descriptor.extent_width, 1);
        assert_eq!(descriptor.extent_height, 0);
        assert_eq!(descriptor.draw_x, 160);
        assert_eq!(descriptor.draw_y, 100);
        assert_eq!(
            descriptor.flags,
            SHIP_3D_OBJECT_VISIBLE_FLAG
                | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG
        );
    }

    #[test]
    fn sprite_slot_position_update_marks_dirty_only_when_active_and_changed() {
        let mut inactive = Ship3dObjectSpriteDescriptor {
            draw_x: 1,
            draw_y: 2,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            update_ship_3d_sprite_slot_position(&mut inactive, 3, 4),
            Ship3dSpriteSlotUpdateEffect::default()
        );
        assert_eq!(inactive.draw_x, 1);
        assert_eq!(inactive.draw_y, 2);

        let mut active = Ship3dObjectSpriteDescriptor {
            flags: 0x0001,
            draw_x: 10,
            draw_y: 20,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            update_ship_3d_sprite_slot_position(&mut active, 10, 21),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                marked_dirty: true,
                updated_position: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(active.draw_x, 10);
        assert_eq!(active.draw_y, 21);
        assert_eq!(
            active.flags,
            SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
        );
    }

    #[test]
    fn sprite_slot_extent_update_matches_binary_dirty_and_bit4_rules() {
        let mut natural = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG,
            source_width: 64,
            source_height: 32,
            extent_width: 65,
            extent_height: 33,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            update_ship_3d_sprite_slot_extent(&mut natural, 64, 32),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                marked_dirty: true,
                cleared_extent_changed_flag: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(
            natural.flags,
            SHIP_3D_OBJECT_VISIBLE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
        );
        assert_eq!(natural.extent_width, 65);
        assert_eq!(natural.extent_height, 33);

        assert_eq!(
            update_ship_3d_sprite_slot_extent(&mut natural, 80, 40),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                marked_dirty: true,
                updated_extent: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(
            natural.flags,
            SHIP_3D_OBJECT_VISIBLE_FLAG
                | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG
        );
        assert_eq!(natural.extent_width, 80);
        assert_eq!(natural.extent_height, 40);
    }

    #[test]
    fn sprite_slot_dirty_commit_copies_current_geometry_for_active_dirty_slots() {
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 10,
            draw_y: 20,
            extent_width: 30,
            extent_height: 40,
            committed_draw_x: 1,
            committed_draw_y: 2,
            committed_extent_width: 3,
            committed_extent_height: 4,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            commit_ship_3d_sprite_slot_dirty_geometry(&mut descriptor),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                committed_geometry: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(descriptor.committed_draw_x, 10);
        assert_eq!(descriptor.committed_draw_y, 20);
        assert_eq!(descriptor.committed_extent_width, 30);
        assert_eq!(descriptor.committed_extent_height, 40);
        assert_eq!(
            descriptor.flags,
            SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
        );
    }

    #[test]
    fn sprite_slot_dirty_commit_skips_clean_or_inactive_slots() {
        let mut clean = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG,
            draw_x: 10,
            committed_draw_x: 1,
            ..Ship3dObjectSpriteDescriptor::default()
        };
        assert_eq!(
            commit_ship_3d_sprite_slot_dirty_geometry(&mut clean),
            Ship3dSpriteSlotUpdateEffect::default()
        );
        assert_eq!(clean.committed_draw_x, 1);

        let mut inactive_dirty = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 10,
            committed_draw_x: 1,
            ..Ship3dObjectSpriteDescriptor::default()
        };
        assert_eq!(
            commit_ship_3d_sprite_slot_dirty_geometry(&mut inactive_dirty),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(inactive_dirty.committed_draw_x, 1);
    }

    #[test]
    fn dirty_rect_clip_snapshot_replaces_list_and_clears_flag() {
        let mut dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 1,
                right: 2,
                top: 3,
                bottom: 4,
            }],
            sentinel: 0,
        };
        let mut snapshot_armed = true;
        let clip = Ship3dProjectionViewport {
            left: 5,
            right: 100,
            top: 0x23,
            bottom: 0xa5,
        };

        assert_eq!(
            commit_ship_3d_global_clip_snapshot(&mut dirty_rects, &mut snapshot_armed, clip),
            Ship3dDirtyRectSnapshotEffect {
                ran: true,
                wrote_clip_rect: true,
                wrote_sentinel: true,
                cleared_snapshot_flag: true,
            }
        );
        assert!(!snapshot_armed);
        assert_eq!(
            dirty_rects,
            Ship3dDirtyRectList {
                rects: vec![clip],
                sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
            }
        );
    }

    #[test]
    fn dirty_rect_clip_snapshot_without_flag_is_noop() {
        let mut dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 1,
                right: 2,
                top: 3,
                bottom: 4,
            }],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };
        let original = dirty_rects.clone();
        let mut snapshot_armed = false;

        assert_eq!(
            commit_ship_3d_global_clip_snapshot(
                &mut dirty_rects,
                &mut snapshot_armed,
                Ship3dProjectionViewport {
                    left: 5,
                    right: 100,
                    top: 0x23,
                    bottom: 0xa5,
                },
            ),
            Ship3dDirtyRectSnapshotEffect::default()
        );
        assert!(!snapshot_armed);
        assert_eq!(dirty_rects, original);
    }

    #[test]
    fn dirty_sprite_slot_render_walk_collects_intersections_descending_and_clears_dirty() {
        let mut slots = vec![
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
                    | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                    | 0x0008
                    | 0x0100,
                draw_x: 1,
                draw_y: 1,
                extent_width: 4,
                extent_height: 4,
                ..Ship3dObjectSpriteDescriptor::default()
            },
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
                draw_x: 10,
                draw_y: 10,
                extent_width: 4,
                extent_height: 4,
                ..Ship3dObjectSpriteDescriptor::default()
            },
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
                    | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                    | 0x000c
                    | 0x0020
                    | 0x0040
                    | 0x0300,
                draw_x: 20,
                draw_y: 20,
                extent_width: 8,
                extent_height: 8,
                ..Ship3dObjectSpriteDescriptor::default()
            },
        ];
        let dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 0,
                right: 30,
                top: 0,
                bottom: 30,
            }],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };

        let commands =
            collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &dirty_rects, 0, 2);

        assert_eq!(
            commands,
            vec![
                Ship3dSpriteSlotRenderCommand {
                    slot_index: 2,
                    dispatch_index: 7,
                    destination_remap_mode: 3,
                    flip_x: true,
                    flip_y: true,
                    slot_rect: Ship3dProjectionViewport {
                        left: 20,
                        right: 28,
                        top: 20,
                        bottom: 28,
                    },
                    dirty_rect: dirty_rects.rects[0],
                },
                Ship3dSpriteSlotRenderCommand {
                    slot_index: 0,
                    dispatch_index: 5,
                    destination_remap_mode: 1,
                    flip_x: false,
                    flip_y: false,
                    slot_rect: Ship3dProjectionViewport {
                        left: 1,
                        right: 5,
                        top: 1,
                        bottom: 5,
                    },
                    dirty_rect: dirty_rects.rects[0],
                },
            ]
        );
        assert_eq!(slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
        assert_eq!(slots[1].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
        assert_eq!(slots[2].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
    }

    #[test]
    fn dirty_sprite_slot_render_walk_without_dirty_rects_is_noop() {
        let mut slots = vec![Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 1,
            draw_y: 1,
            extent_width: 4,
            extent_height: 4,
            ..Ship3dObjectSpriteDescriptor::default()
        }];

        assert_eq!(
            collect_ship_3d_dirty_sprite_slot_render_commands(
                &mut slots,
                &Ship3dDirtyRectList::default(),
                0,
                0,
            ),
            Vec::<Ship3dSpriteSlotRenderCommand>::new()
        );
        assert_ne!(slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
    }

    #[test]
    fn dirty_sprite_slot_render_walk_uses_exclusive_edges() {
        let mut slots = vec![Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 10,
            draw_y: 10,
            extent_width: 5,
            extent_height: 5,
            ..Ship3dObjectSpriteDescriptor::default()
        }];
        let dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 15,
                right: 30,
                top: 10,
                bottom: 30,
            }],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };

        assert_eq!(
            collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &dirty_rects, 0, 0),
            Vec::<Ship3dSpriteSlotRenderCommand>::new()
        );
        assert_eq!(slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
    }

    #[test]
    fn temp_snd_setup_without_trigger_is_noop() {
        let mut state = Ship3dTempSndState {
            phase: 1,
            plane_copy_enabled: false,
            scene_selector: 0x1234,
            hold_ticks: 0x0055,
            setup_flag_a: true,
            setup_flag_b: true,
            ..Ship3dTempSndState::default()
        };

        let effect = run_ship_3d_temp_snd_setup(&mut state).unwrap();

        assert_eq!(effect, Ship3dTempSndEffect::default());
        assert_eq!(
            state,
            Ship3dTempSndState {
                phase: 1,
                plane_copy_enabled: false,
                scene_selector: 0x1234,
                hold_ticks: 0x0055,
                setup_flag_a: true,
                setup_flag_b: true,
                ..Ship3dTempSndState::default()
            }
        );
    }

    #[test]
    fn temp_snd_setup_cycles_phase_and_runs_sequence_branch() {
        let mut state = Ship3dTempSndState {
            trigger: true,
            auxiliary_trigger: true,
            phase: 0,
            sequence_active: true,
            plane_copy_enabled: false,
            scene_selector: 0x2222,
            hold_ticks: 0x0040,
            setup_flag_a: true,
            setup_flag_b: true,
            ..Ship3dTempSndState::default()
        };

        let effect = run_ship_3d_temp_snd_setup(&mut state).unwrap();

        assert_eq!(
            effect,
            Ship3dTempSndEffect {
                ran: true,
                selected_callback_offset: Some(0x0087),
                next_phase: Some(1),
                load_snd_bank_path: Some(SHIP_3D_TEMP_SND_PATH_OFFSET),
                restore_snd_bank_path: Some(SHIP_3D_TB_SND_PATH_OFFSET),
                preserved_mouse_position: true,
                reset_callback_bank_gate: true,
                called_presentation_callback: true,
                reset_hold_ticks: true,
                wrote_viewport_descriptor: true,
                sequence_branch: true,
                temporarily_disabled_plane_copy: true,
                enabled_plane_copy: true,
                reset_scene_selector: true,
                ..Ship3dTempSndEffect::default()
            }
        );
        assert!(!state.trigger);
        assert!(!state.auxiliary_trigger);
        assert_eq!(state.phase, 1);
        assert!(state.plane_copy_enabled);
        assert_eq!(
            state.scene_selector,
            SHIP_3D_TEMP_SND_SCENE_SELECTOR_SENTINEL
        );
        assert_eq!(state.hold_ticks, 0);
        assert!(state.fullscreen_refresh);
        assert_eq!(
            state.viewport_descriptor,
            SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR
        );
        assert!(state.setup_flag_a);
        assert!(state.setup_flag_b);
    }

    #[test]
    fn temp_snd_setup_wraps_phase_and_runs_non_sequence_branch() {
        let mut state = Ship3dTempSndState {
            trigger: true,
            auxiliary_trigger: true,
            phase: 2,
            sequence_active: false,
            plane_copy_enabled: false,
            scene_selector: 0x3333,
            hold_ticks: 0x0040,
            setup_flag_a: true,
            setup_flag_b: true,
            ..Ship3dTempSndState::default()
        };

        let effect = run_ship_3d_temp_snd_setup(&mut state).unwrap();

        assert_eq!(
            effect,
            Ship3dTempSndEffect {
                ran: true,
                selected_callback_offset: Some(0x009c),
                next_phase: Some(0),
                load_snd_bank_path: Some(SHIP_3D_TEMP_SND_PATH_OFFSET),
                restore_snd_bank_path: Some(SHIP_3D_TB_SND_PATH_OFFSET),
                preserved_mouse_position: true,
                reset_callback_bank_gate: true,
                called_presentation_callback: true,
                reset_hold_ticks: true,
                wrote_viewport_descriptor: true,
                non_sequence_branch: true,
                reset_setup_flags: true,
                ..Ship3dTempSndEffect::default()
            }
        );
        assert!(!state.trigger);
        assert!(!state.auxiliary_trigger);
        assert_eq!(state.phase, 0);
        assert!(!state.plane_copy_enabled);
        assert_eq!(state.scene_selector, 0x3333);
        assert_eq!(state.hold_ticks, 0);
        assert!(state.fullscreen_refresh);
        assert_eq!(
            state.viewport_descriptor,
            SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR
        );
        assert!(!state.setup_flag_a);
        assert!(!state.setup_flag_b);
    }

    #[test]
    fn navigation_final_reset_without_exit_pending_is_noop() {
        let mut state = Ship3dNavigationFinalResetState {
            hud_flags: 0xaaaa,
            status_flags: 0xff,
            scroll_mode: 0x1234,
            ..Ship3dNavigationFinalResetState::default()
        };

        let effect = run_ship_3d_navigation_final_reset(&mut state);

        assert_eq!(effect, Ship3dNavigationFinalResetEffect::default());
        assert_eq!(
            state,
            Ship3dNavigationFinalResetState {
                hud_flags: 0xaaaa,
                status_flags: 0xff,
                scroll_mode: 0x1234,
                ..Ship3dNavigationFinalResetState::default()
            }
        );
    }

    #[test]
    fn navigation_final_reset_reenters_active_sequence_while_opening() {
        let mut state = Ship3dNavigationFinalResetState {
            exit_pending: true,
            opening: true,
            hud_flags: 0xaaaa,
            status_flags: 0xff,
            scroll_mode: 0x1234,
            ..Ship3dNavigationFinalResetState::default()
        };

        let effect = run_ship_3d_navigation_final_reset(&mut state);

        assert_eq!(
            effect,
            Ship3dNavigationFinalResetEffect {
                reentered_active_sequence: true,
                ..Ship3dNavigationFinalResetEffect::default()
            }
        );
        assert_eq!(
            state,
            Ship3dNavigationFinalResetState {
                exit_pending: true,
                opening: true,
                hud_flags: 0xaaaa,
                status_flags: 0xff,
                scroll_mode: 0x1234,
                ..Ship3dNavigationFinalResetState::default()
            }
        );
    }

    #[test]
    fn navigation_final_reset_applies_binary_teardown_state() {
        let mut state = Ship3dNavigationFinalResetState {
            exit_pending: true,
            hud_flags: 0x1111,
            nav_choice_hold_ticks: 0x2222,
            nav_choice_timer: 0x3333,
            dialogue_state: 0x4444,
            scene_band_top: 0x5555,
            scene_selector: 0x6666,
            active_record: 0x7777,
            presentation_gate: true,
            pending_state_byte: true,
            subtitle_gate: true,
            presentation_defer_active: true,
            secondary_presentation_defer_active: true,
            plane_copy_enabled: true,
            sequence_active: true,
            status_flags: 0xff,
            secondary_status_flag: true,
            dirty_marker: 0x12,
            scroll_value: 0x8888,
            scroll_mode: 0x9999,
            ..Ship3dNavigationFinalResetState::default()
        };

        let effect = run_ship_3d_navigation_final_reset(&mut state);

        assert_eq!(
            effect,
            Ship3dNavigationFinalResetEffect {
                ran: true,
                cleared_dialogue_state: true,
                reset_hud_state: true,
                reset_presentation_gates: true,
                reset_sequence_flags: true,
                reset_status_flags: true,
                copied_backbuffer_restore_block: true,
                cleared_overlay_scratch: true,
                reset_scroll_state: true,
                called_render_clear: true,
                called_input_reset: true,
                called_target_cleanup: true,
                ..Ship3dNavigationFinalResetEffect::default()
            }
        );
        assert_eq!(state.hud_flags, SHIP_3D_FINAL_RESET_HUD_FLAGS);
        assert_eq!(state.nav_choice_hold_ticks, 0);
        assert_eq!(state.nav_choice_timer, SHIP_3D_FINAL_RESET_NAV_TIMER);
        assert!(state.post_reset_gate);
        assert!(state.navigation_gate);
        assert_eq!(state.dialogue_state, 0);
        assert_eq!(state.scene_band_top, 0);
        assert_eq!(state.scene_selector, SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL);
        assert_eq!(
            state.active_record,
            SHIP_3D_FINAL_RESET_ACTIVE_RECORD_SENTINEL
        );
        assert!(!state.presentation_gate);
        assert!(!state.exit_pending);
        assert!(!state.pending_state_byte);
        assert!(!state.subtitle_gate);
        assert!(!state.presentation_defer_active);
        assert!(!state.secondary_presentation_defer_active);
        assert!(!state.plane_copy_enabled);
        assert!(!state.sequence_active);
        assert_eq!(state.status_flags, SHIP_3D_FINAL_RESET_STATUS_FLAG_MASK);
        assert!(!state.secondary_status_flag);
        assert_eq!(state.dirty_marker, SHIP_3D_FINAL_RESET_DIRTY_MARKER);
        assert_eq!(state.scroll_value, 0);
        assert_eq!(state.scroll_mode, SHIP_3D_FINAL_RESET_SCROLL_MODE);
    }

    #[test]
    fn navigation_sequence_active_path_runs_temp_snd_and_blocks_on_presentation() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            true,
            false,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                blocked_by_presentation_active: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(!state.framebuffer_dirty);
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_copies_framebuffer_without_target_query_when_duration_differs() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION - 1,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(&mut state, false, false, true, 0);

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.framebuffer_dirty);
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_waits_while_interpolation_is_active() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(&mut state, false, false, false, 0);

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                interpolation_active: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.framebuffer_dirty);
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_complete_selection_arms_exit_pending() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(&mut state, false, false, true, 0);

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                queried_target_list: true,
                armed_exit_pending: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.framebuffer_dirty);
        assert!(!state.sequence_active);
        assert!(state.exit_pending);
    }

    #[test]
    fn navigation_sequence_complete_no_selection_keeps_sequence_active() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            false,
            false,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                queried_target_list: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_inactive_without_defer_arms_opening_exit() {
        let mut state = Ship3dNavigationSequenceState::default();

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            false,
            false,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                armed_opening_exit: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.exit_pending);
        assert!(state.opening);
    }

    #[test]
    fn navigation_sequence_exit_pending_without_opening_reports_final_reset() {
        let mut state = Ship3dNavigationSequenceState {
            exit_pending: true,
            opening: false,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            false,
            false,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                final_reset_pending: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.exit_pending);
        assert!(!state.opening);
    }

    fn nav_record(
        offset: u16,
        kind_flags: u16,
        state_flags: u8,
        counter_link: u16,
        related_target: u16,
    ) -> Ship3dNavigationRuntimeRecord {
        Ship3dNavigationRuntimeRecord {
            offset,
            kind_flags,
            state_flags,
            counter_link,
            related_target,
            source_parent: None,
        }
    }

    fn nav_record_with_source_parent(
        offset: u16,
        kind_flags: u16,
        state_flags: u8,
        counter_link: u16,
        related_target: u16,
        source_parent: Option<u16>,
    ) -> Ship3dNavigationRuntimeRecord {
        Ship3dNavigationRuntimeRecord {
            offset,
            kind_flags,
            state_flags,
            counter_link,
            related_target,
            source_parent,
        }
    }

    #[test]
    fn navigation_source_records_follow_selector_11_tree_depth_first() {
        let records = [
            nav_record_with_source_parent(0x3000, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3100, 0, 0, 0, 0, Some(0x3000)),
            nav_record_with_source_parent(0x3200, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3300, 0, 0, 0, 0, Some(0x9999)),
        ];
        let source_entries = [
            Ship3dNavigationSourceEntry {
                record_offset: 0x3000,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3100,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3200,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3300,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3400,
                entry_kind: 0,
            },
        ];

        let source_records =
            build_ship_3d_navigation_source_records(&source_entries, &records, 0x2000).unwrap();

        assert_eq!(
            source_records,
            vec![0x3000, 0x3100, 0x3200, SHIP_3D_TARGET_EXIT_SENTINEL]
        );
    }

    #[test]
    fn navigation_source_records_stop_before_first_non_kind1_next_entry() {
        let records = [
            nav_record_with_source_parent(0x3000, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3100, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3200, 0, 0, 0, 0, Some(0x2000)),
        ];
        let source_entries = [
            Ship3dNavigationSourceEntry {
                record_offset: 0x3000,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3100,
                entry_kind: 0,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3200,
                entry_kind: 1,
            },
        ];

        let source_records =
            build_ship_3d_navigation_source_records(&source_entries, &records, 0x2000).unwrap();

        assert_eq!(source_records, vec![0x3000, SHIP_3D_TARGET_EXIT_SENTINEL]);
    }

    #[test]
    fn navigation_source_records_skip_kinds_without_selector_11_parent() {
        let records = [
            nav_record(0x3000, 0, 0, 0, 0),
            nav_record_with_source_parent(0x3100, 0, 0, 0, 0, Some(0x2000)),
        ];
        let source_entries = [
            Ship3dNavigationSourceEntry {
                record_offset: 0x3000,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3100,
                entry_kind: 1,
            },
        ];

        let source_records =
            build_ship_3d_navigation_source_records(&source_entries, &records, 0x2000).unwrap();

        assert_eq!(source_records, vec![0x3100, SHIP_3D_TARGET_EXIT_SENTINEL]);
    }

    #[test]
    fn navigation_source_records_require_at_least_one_source_entry() {
        assert_eq!(
            build_ship_3d_navigation_source_records(&[], &[], 0x2000),
            None
        );
    }

    fn position_record(
        offset: u16,
        kind_flags: u16,
        parent_link: Option<u16>,
        kind100_match_word: Option<u16>,
        kind100_relation_word: Option<u16>,
    ) -> Ship3dPositionRecord {
        Ship3dPositionRecord {
            offset,
            kind_flags,
            parent_link,
            kind100_match_word,
            kind100_relation_word,
        }
    }

    fn position_field(offset: u16, x: u16, y: u16) -> Ship3dPositionField {
        Ship3dPositionField { offset, x, y }
    }

    #[test]
    fn position_field_resolves_direct_coordinate_kinds() {
        let records = [
            position_record(
                0x1000,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                None,
                None,
                None,
            ),
            position_record(
                0x1100,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10,
                None,
                None,
                None,
            ),
            position_record(
                0x1200,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40,
                None,
                None,
                None,
            ),
            position_record(
                0x1300,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200,
                None,
                None,
                None,
            ),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            Some(0x1018)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1100, 0x2000, 0),
            Some(0x1118)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1200, 0x2000, 0),
            Some(0x1200)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1300, 0x2000, 0),
            Some(0x1306)
        );
    }

    #[test]
    fn position_field_follows_selector_11_parent_chain() {
        let records = [
            position_record(0x1000, 0x0002, Some(0x1100), None, None),
            position_record(0x1100, 0x0002, Some(0x1200), None, None),
            position_record(
                0x1200,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                None,
                None,
                None,
            ),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            Some(0x1218)
        );
    }

    #[test]
    fn position_field_uses_arche_for_selector_11_sentinel() {
        let records = [
            position_record(0x1000, 0x0002, None, None, None),
            position_record(
                0x2000,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10,
                None,
                None,
                None,
            ),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            Some(0x2018)
        );
    }

    #[test]
    fn position_field_kind100_chooses_match_or_mismatch_block() {
        let records = [position_record(
            0x1000,
            SHIP_3D_OBJECT_KIND_POSITION_KIND100,
            None,
            Some(0x2222),
            None,
        )];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0x2222),
            Some(0x1018)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0x3333),
            Some(0x101c)
        );
    }

    #[test]
    fn position_field_rejects_unresolvable_parent_chain() {
        let records = [
            position_record(0x1000, 0x0002, Some(0x1000), None, None),
            position_record(0x2000, 0x0020, Some(0x1000), None, None),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            None
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x2000, 0x1000, 0),
            None
        );
    }

    #[test]
    fn position_distance_uses_binary_sqrt_distance() {
        let first = position_field(0x1000, 10, 20);
        let second = position_field(0x2000, 13, 24);

        assert_eq!(ship_3d_position_field_distance(first, second), Some(5));
    }

    #[test]
    fn position_distance_uses_binary_rounded_sqrt() {
        assert_eq!(ship_3d_binary_sqrt(24), Some(5));
        assert_eq!(ship_3d_binary_sqrt(20), Some(4));
        assert_eq!(
            ship_3d_position_field_distance(
                position_field(0x1000, 0, 0),
                position_field(0x2000, 2, 4),
            ),
            Some(4)
        );
    }

    #[test]
    fn position_distance_uses_wrapping_signed_word_diffs() {
        assert_eq!(
            ship_3d_position_field_distance(
                position_field(0x1000, 0xffff, 0),
                position_field(0x2000, 0x0001, 0),
            ),
            Some(2)
        );
    }

    #[test]
    fn position_distance_resolves_kind100_against_other_relation_word() {
        let first = position_record(
            0x1000,
            SHIP_3D_OBJECT_KIND_POSITION_KIND100,
            None,
            Some(0x2222),
            None,
        );
        let second_match = position_record(
            0x2000,
            SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
            None,
            None,
            Some(0x2222),
        );
        let second_mismatch = position_record(
            0x2100,
            SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
            None,
            None,
            Some(0x3333),
        );
        let fields = [
            position_field(0x1018, 0, 0),
            position_field(0x101c, 10, 0),
            position_field(0x2018, 3, 4),
            position_field(0x2118, 3, 4),
        ];

        assert_eq!(
            ship_3d_position_distance(&[first, second_match], &fields, 0x1000, 0x2000, 0, 0),
            Some(5)
        );
        assert_eq!(
            ship_3d_position_distance(&[first, second_mismatch], &fields, 0x1000, 0x2100, 0, 0),
            Some(8)
        );
    }

    #[test]
    fn position_distance_follows_parent_chain_with_inherited_kind100_compare_word() {
        let records = [
            position_record(0x1000, 0x0002, Some(0x1100), None, None),
            position_record(
                0x1100,
                SHIP_3D_OBJECT_KIND_POSITION_KIND100,
                None,
                Some(0x4444),
                None,
            ),
            position_record(
                0x2000,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                None,
                None,
                None,
            ),
        ];
        let fields = [
            position_field(0x1118, 0, 0),
            position_field(0x111c, 10, 0),
            position_field(0x2018, 3, 4),
        ];

        assert_eq!(
            ship_3d_position_distance(&records, &fields, 0x1000, 0x2000, 0, 0x4444),
            Some(5)
        );
        assert_eq!(
            ship_3d_position_distance(&records, &fields, 0x1000, 0x2000, 0, 0x5555),
            Some(8)
        );
    }

    #[test]
    fn object_table_bit_test_uses_selector5_kind2_field_offset() {
        assert_eq!(
            vm::vm_field_offset(SHIP_3D_SOURCE_BITSET_SELECTOR, SHIP_3D_SOURCE_BITSET_KIND),
            Some(0x1e)
        );
    }

    #[test]
    fn object_table_bit_test_uses_high_bit_first_masks() {
        let object_table = [
            0x1000, 0x1014, 0x1028, 0x103c, 0x1050, 0x1064, 0x1078, 0x108c, 0x10a0,
        ];
        let mut bitset = [0u8; 0x21];
        bitset[0x1e] = 0x81;
        bitset[0x1f] = 0x80;

        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x1000),
            Some(true)
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x1014),
            Some(false)
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x108c),
            Some(true)
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x10a0),
            Some(true)
        );
    }

    #[test]
    fn object_table_bit_test_requires_known_object_and_available_byte() {
        let object_table = [0x1000, 0x1014];
        let bitset = [0xffu8; 0x1f];

        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x9999),
            None
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset[..0x1e], 0x1000),
            None
        );
    }

    #[test]
    fn c1_source_selection_accepts_kind2_when_operand_bit_is_set() {
        let records = [nav_record(0x3000, SHIP_3D_C1_SOURCE_KIND_BITSET, 0, 0, 0)];
        let object_table = [0x2000];
        let mut source_list_bytes = [0u8; 0x21];
        source_list_bytes[0x20] = 0x80;

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            Some(Some(0x3000))
        );
    }

    #[test]
    fn c1_source_selection_falls_through_clear_bit_to_kind1_operand_flag() {
        let records = [
            nav_record(0x3000, SHIP_3D_C1_SOURCE_KIND_BITSET, 0, 0, 0),
            nav_record(0x3100, SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG, 0, 0, 0),
        ];
        let object_table = [0x2000];
        let source_list_bytes = [0u8; 0x21];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, 0x3100, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG,
            ),
            Some(Some(0x3100))
        );
    }

    #[test]
    fn c1_source_selection_uses_current_source_cursor_for_kind2_bitset() {
        let records = [
            nav_record(0x3000, 0x0003, 0, 0, 0),
            nav_record(0x3100, SHIP_3D_C1_SOURCE_KIND_BITSET, 0, 0, 0),
        ];
        let object_table = [0x2000];
        let mut source_list_bytes = [0u8; 0x23];
        source_list_bytes[0x20] = 0x00;
        source_list_bytes[0x22] = 0x80;

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, 0x3100, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            Some(Some(0x3100))
        );
    }

    #[test]
    fn c1_source_selection_reaches_sentinel_without_match() {
        let records = [nav_record(
            0x3000,
            SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG,
            0,
            0,
            0,
        )];
        let object_table = [0x2000];
        let source_list_bytes = [0xffu8; 0x1f];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x9999],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            Some(None)
        );
    }

    #[test]
    fn c1_source_selection_requires_known_records_and_sentinel() {
        let records = [nav_record(
            0x3000,
            SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG,
            0,
            0,
            0,
        )];
        let object_table = [0x2000];
        let source_list_bytes = [0xffu8; 0x1f];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x9999, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG,
            ),
            None
        );
        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            None
        );
    }

    #[test]
    fn c1_kind10_destination_uses_selector13_kind10_field_offset() {
        assert_eq!(
            vm::vm_field_offset(
                SHIP_3D_C1_DESTINATION_SELECTOR,
                SHIP_3D_C1_KIND10_RECORD_KIND
            ),
            Some(0x1c)
        );
        assert_eq!(
            resolve_ship_3d_c1_kind10_destination_record(0x4000, SHIP_3D_C1_KIND10_RECORD_KIND,),
            Some(0x401c)
        );
        assert_eq!(
            resolve_ship_3d_c1_kind10_destination_record(0x4000, 0x0002),
            None
        );
    }

    #[test]
    fn c1_kind10_destination_write_records_c1_operand_and_aux2() {
        let mut slot = Ship3dRecordStateSlot::default();

        let write = write_ship_3d_c1_kind10_destination_slot(
            0x4000,
            SHIP_3D_C1_KIND10_RECORD_KIND,
            &mut slot,
            0x2000,
        );

        let expected_slot = Ship3dRecordStateSlot {
            opcode: SHIP_3D_C1_RECORD_STATE_OPCODE,
            operand: 0x2000,
            aux_word: SHIP_3D_C1_RECORD_STATE_AUX_WORD,
        };
        assert_eq!(
            write,
            Some(Some(Ship3dC1DestinationWrite {
                destination_record_offset: 0x401c,
                slot: expected_slot,
            }))
        );
        assert_eq!(slot, expected_slot);
    }

    #[test]
    fn c1_kind10_destination_write_branches_when_destination_occupied() {
        let mut slot = Ship3dRecordStateSlot {
            opcode: 0x00c4,
            operand: 0x1111,
            aux_word: 0x2222,
        };

        let write = write_ship_3d_c1_kind10_destination_slot(
            0x4000,
            SHIP_3D_C1_KIND10_RECORD_KIND,
            &mut slot,
            0x2000,
        );

        assert_eq!(write, Some(None));
        assert_eq!(
            slot,
            Ship3dRecordStateSlot {
                opcode: 0x00c4,
                operand: 0x1111,
                aux_word: 0x2222,
            }
        );
    }

    #[test]
    fn c1_kind10_destination_write_checks_only_first_destination_word() {
        let mut slot = Ship3dRecordStateSlot {
            opcode: 0,
            operand: 0x1111,
            aux_word: 0x2222,
        };

        assert_eq!(
            write_ship_3d_c1_kind10_destination_slot(
                0x4000,
                SHIP_3D_C1_KIND10_RECORD_KIND,
                &mut slot,
                0x2000,
            )
            .map(|write| write.map(|write| write.destination_record_offset)),
            Some(Some(0x401c))
        );
        assert_eq!(
            slot,
            Ship3dRecordStateSlot {
                opcode: SHIP_3D_C1_RECORD_STATE_OPCODE,
                operand: 0x2000,
                aux_word: SHIP_3D_C1_RECORD_STATE_AUX_WORD,
            }
        );
    }

    #[test]
    fn navigation_candidates_filter_kind2_active_records_and_skip_honk() {
        let records = [
            nav_record(0x1000, SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE, 0x01, 0, 0),
            nav_record(0x1100, SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE, 0x00, 0, 0),
            nav_record(0x1200, 0x0003, 0x01, 0, 0),
            nav_record(0x1300, SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE, 0x01, 0, 0),
        ];

        let candidates = build_ship_3d_navigation_candidate_records(
            &[0x1000, 0x1100, 0x1200, 0x1300, 0xffff, 0x1400],
            &records,
            0x1300,
        )
        .unwrap();

        assert_eq!(candidates, vec![0x1000]);
    }

    #[test]
    fn navigation_candidates_require_source_sentinel() {
        let records = [nav_record(
            0x1000,
            SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
            0x01,
            0,
            0,
        )];

        assert_eq!(
            build_ship_3d_navigation_candidate_records(&[0x1000], &records, 0),
            None
        );
    }

    #[test]
    fn navigation_trigger_defers_first_matching_c4_candidate() {
        let records = [
            nav_record(0x2000, 0x0000, 0x00, 0x2550, 0),
            nav_record(
                0x3100,
                SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
                0x01,
                0,
                0x2000,
            ),
        ];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            render_clip_bottom: 0x9999,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0x3100, 0xffff],
            0x6754,
            0x6758,
            0x0007,
            [0x10, 0x20, 0x30, 0x40],
        )
        .unwrap();

        assert_eq!(
            effect,
            Ship3dNavigationTriggerEffect {
                candidate_records: vec![0x3100],
                copied_pending_presentation_state: true,
                incremented_counter_record: Some(0x2000),
                deferred_record_type: Some(SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE),
                deferred_record_related: Some(0x3100),
                candidate_handler_record: Some(0x3104),
                cleared_trigger: true,
                started_sequence: true,
                set_scene_band: true,
                restored_render_clip: true,
                cleared_active_dialogue_record: true,
                requested_closing: true,
                ..Ship3dNavigationTriggerEffect::default()
            }
        );
        assert!(!state.trigger_active);
        assert!(state.sequence_active);
        assert_eq!(state.requested_presentation_state, 0x0007);
        assert_eq!(state.scene_band_top, SHIP_3D_NAVIGATION_SCENE_BAND_TOP);
        assert_eq!(
            state.render_clip_bottom,
            SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM
        );
        assert_eq!(state.active_dialogue_record, SHIP_3D_TARGET_EXIT_SENTINEL);
        assert!(state.closing);
        assert_eq!(state.depth_step, SHIP_3D_NAVIGATION_TRIGGER_CLOSE_STEP);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn navigation_trigger_match_any_flag_ignores_candidate_related_target() {
        let records = [
            nav_record(
                0x2000,
                0x0000,
                SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG,
                0x2550,
                0,
            ),
            nav_record(
                0x3100,
                SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
                0x01,
                0,
                0x9999,
            ),
        ];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0x3100, 0xffff],
            0x6754,
            0x6758,
            0,
            [0; SHIP_3D_INTERPOLATION_WORDS],
        )
        .unwrap();

        assert_eq!(effect.deferred_record_related, Some(0x3100));
        assert!(!effect.opened_target_list);
    }

    #[test]
    fn navigation_trigger_ark_related_candidate_opens_target_list() {
        let records = [
            nav_record(0x2000, 0x0000, 0x00, 0, 0),
            nav_record(
                0x3100,
                SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
                0x01,
                0,
                0x6758,
            ),
        ];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            layout_rect_snapshot: [0xaaaa, 0xbbbb, 0xcccc, 0xdddd],
            interpolation_current_tick: 5,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0x3100, 0xffff],
            0x6754,
            0x6758,
            0,
            [0x10, 0x20, 0x30, 0x40],
        )
        .unwrap();

        assert_eq!(
            effect,
            Ship3dNavigationTriggerEffect {
                candidate_records: vec![0x3100],
                copied_pending_presentation_state: true,
                incremented_counter_record: Some(0x2000),
                opened_target_list: true,
                reset_interpolation_tick: true,
                ran_layout_prepass: true,
                copied_layout_x_and_width: true,
                cleared_trigger: true,
                started_sequence: true,
                set_scene_band: true,
                restored_render_clip: true,
                cleared_active_dialogue_record: true,
                requested_closing: true,
                ..Ship3dNavigationTriggerEffect::default()
            }
        );
        assert_eq!(state.hud_flags, SHIP_3D_NAVIGATION_TARGET_LIST_FLAG);
        assert_eq!(
            state.interpolation_duration_ticks,
            SHIP_3D_NAVIGATION_INTERPOLATION_DURATION
        );
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(state.layout_rect_snapshot, [0x10, 0xbbbb, 0x30, 0xdddd]);
        assert!(!state.target_query_mode);
    }

    #[test]
    fn navigation_trigger_no_candidate_opens_target_list_and_redirects_counter_increment() {
        let records = [nav_record(
            0x2000,
            SHIP_3D_NAVIGATION_REDIRECT_COUNTER_FLAG,
            0x00,
            0x2a00,
            0,
        )];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0xffff],
            0x6754,
            0x6758,
            0,
            [0x10, 0x20, 0x30, 0x40],
        )
        .unwrap();

        assert_eq!(effect.candidate_records, Vec::<u16>::new());
        assert_eq!(effect.incremented_counter_record, Some(0x2a00));
        assert!(effect.opened_target_list);
        assert_eq!(state.hud_flags, SHIP_3D_NAVIGATION_TARGET_LIST_FLAG);
    }

    fn axis_aligned_projection_matrix() -> Ship3dProjectionMatrix {
        // Rows 1/2/3 pick x, y, z directly at ~unit Q15 scale so a translated
        // point (tx,ty,tz>0) projects near screen centre with depth ~= tz.
        Ship3dProjectionMatrix {
            terms: [
                0x7fff, 0, 0, // screen-x numerator uses tx
                0, 0x7fff, 0, // screen-y numerator uses ty
                0, 0, 0x7fff, // depth uses tz
            ],
        }
    }

    #[test]
    fn render_point_cloud_matches_manual_primitive_loop_and_writes_once() {
        let matrix = axis_aligned_projection_matrix();
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: SHIP_3D_PROJECTION_SCREEN_WIDTH as u16,
            top: 0,
            bottom: SHIP_3D_PROJECTION_SCREEN_HEIGHT as u16,
        };

        // A spread of points, including the on-axis (0,0,z) point that projects
        // to screen centre, and a duplicate of it to exercise write-once.
        let p = |x, y, z| Ship3dProjectionPoint { x, y, z };
        let points = vec![
            p(0, 0, 0x0100),
            p(0, 0, 0x0100), // duplicate cell -> write-once
            p(0x40, 0x30, 0x0180),
            p(0x80, 0x20, 0x0200),
            p(0, 0, 0), // depth 0 -> skipped
            p(0x20, 0x60, 0x0140),
        ];

        // Expected buffer/count from calling the primitives directly.
        let mut expected_buffer =
            vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
        let mut expected_plotted = 0usize;
        for &point in &points {
            if let Some(projected) = project_ship_3d_point(point, origin, matrix) {
                if plot_ship_3d_projected_point(&mut expected_buffer, viewport, projected).is_some()
                {
                    expected_plotted += 1;
                }
            }
        }

        let render = render_ship_3d_point_cloud(&points, origin, matrix, viewport);
        assert_eq!(render.plotted, expected_plotted);
        assert_eq!(render.buffer, expected_buffer);

        // The on-axis point must land somewhere, and the duplicate must not be
        // double-counted (write-once): fewer plotted than non-degenerate points.
        assert!(render.plotted >= 1);
        assert!(render.plotted < points.len());
        // Every drawn cell carries a depth shade, never a stray zero.
        assert!(render.buffer.iter().filter(|&&p| p != 0).count() == render.plotted);
    }

    #[test]
    fn blood_prng_first_call_from_zero_state_returns_zero_and_advances_bytes() {
        // Hand-traced from the shipped all-zero state: the 8-iteration carry
        // chain over two zero bytes yields 0, XOR seed 0 stays 0, then the byte
        // advance sets counter=1, b -= 1 (0x00 -> 0xFF), a ^= rol(1,1)=2.
        let mut prng = BloodPrng::default();
        assert_eq!(prng.next(0xffff), 0);
        assert_eq!(
            prng,
            BloodPrng {
                seed_word: 0,
                a: 2,
                b: 0xff,
                counter: 1,
            }
        );
    }

    #[test]
    fn render_ship_3d_starfield_uses_real_table_and_plots_points() {
        // Full faithful path: PRNG -> randomized cloud -> recovered angle table
        // -> camera matrix -> depth-shaded buffer. The point cloud spans the
        // full u16 range, so an origin near its centre keeps points in front of
        // the camera and on screen.
        let mut prng = BloodPrng::seeded_from_rtc_seconds(17);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: 0,
            angle_2f6f: 0,
        };
        let origin = Ship3dProjectionOrigin {
            x: 0x8000,
            y: 0x8000,
            z: 0x8000,
        };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: SHIP_3D_PROJECTION_SCREEN_WIDTH as u16,
            top: 0,
            bottom: SHIP_3D_PROJECTION_SCREEN_HEIGHT as u16,
        };
        let render = render_ship_3d_starfield(&mut prng, angles, origin, viewport).unwrap();
        assert_eq!(
            render.buffer.len(),
            SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT
        );
        // Some points project in front of the camera and shade the buffer, and
        // every drawn cell carries a nonzero depth shade (write-once contract).
        assert!(render.plotted > 0);
        assert_eq!(
            render.buffer.iter().filter(|&&p| p != 0).count(),
            render.plotted
        );
    }

    #[test]
    fn angle_table_matches_binary() {
        // Byte-exact vs the little-endian (cosine, sine) i16 pairs at DS:0x4F45
        // (file 0xD420 + 0x4F45). Skips when the binary is not checked out.
        let candidates = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let base = 0xD420 + 0x4F45;
        for (i, entry) in SHIP_3D_ANGLE_TABLE.iter().enumerate() {
            let off = base + i * 4;
            let cos = i16::from_le_bytes([data[off], data[off + 1]]);
            let sin = i16::from_le_bytes([data[off + 2], data[off + 3]]);
            assert_eq!(entry.cosine, cos, "cosine mismatch at index {i}");
            assert_eq!(entry.sine, sin, "sine mismatch at index {i}");
        }
    }

    #[test]
    fn angle_table_is_a_consistent_trig_table() {
        assert_eq!(SHIP_3D_ANGLE_TABLE.len(), 180);
        // 0deg, 90deg (index 45), 180deg (index 90) at Q14 amplitude 0x4000.
        let entry = |c, s| Ship3dAngleTableEntry { cosine: c, sine: s };
        assert_eq!(SHIP_3D_ANGLE_TABLE[0], entry(0x4000, 0));
        assert_eq!(SHIP_3D_ANGLE_TABLE[45], entry(0, 0x4000));
        assert_eq!(SHIP_3D_ANGLE_TABLE[90], entry(-0x4000, 0));
        // Every entry sits on the Q14 unit circle within rounding.
        for (i, e) in SHIP_3D_ANGLE_TABLE.iter().enumerate() {
            let mag = (i32::from(e.cosine).pow(2) + i32::from(e.sine).pow(2)) as f64;
            assert!(
                (mag.sqrt() - 16384.0).abs() < 2.0,
                "index {i} off the unit circle: {}",
                mag.sqrt()
            );
        }
        // The table feeds the matrix builder without an index-out-of-range.
        let angles = Ship3dMatrixAngles {
            angle_2f71: 10,
            projection_angle_2f6d: 45,
            angle_2f6f: 179,
        };
        assert!(build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles).is_some());
    }

    #[test]
    fn blood_prng_rtc_seed_duplicates_seconds_into_both_seed_bytes() {
        // `mov ah,al` before `mov cs:[0xAEE],ax` puts the seconds byte in both
        // halves of the seed word, and different seconds give different streams.
        assert_eq!(BloodPrng::seeded_from_rtc_seconds(0x2a).seed_word, 0x2a2a);
        assert_eq!(BloodPrng::seeded_from_rtc_seconds(0).seed_word, 0);
        let mut s5 = BloodPrng::seeded_from_rtc_seconds(5);
        let mut s6 = BloodPrng::seeded_from_rtc_seconds(6);
        assert_ne!(
            (0..8).map(|_| s5.next(0xffff)).collect::<Vec<_>>(),
            (0..8).map(|_| s6.next(0xffff)).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn blood_prng_is_deterministic_for_a_given_seed() {
        let mut lhs = BloodPrng {
            seed_word: 0x1234,
            a: 0x9a,
            b: 0x57,
            counter: 3,
        };
        let mut rhs = lhs;
        let lhs_seq: Vec<u16> = (0..64).map(|_| lhs.next(0xffff)).collect();
        let rhs_seq: Vec<u16> = (0..64).map(|_| rhs.next(0xffff)).collect();
        assert_eq!(lhs_seq, rhs_seq);
        assert_eq!(lhs, rhs);
        // A non-trivial seed must actually produce variation, not a constant.
        assert!(
            lhs_seq
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 1
        );
    }

    #[test]
    fn blood_prng_respects_modulus_range() {
        let mut prng = BloodPrng {
            seed_word: 0xbeef,
            a: 0x11,
            b: 0x22,
            counter: 0,
        };
        for modulus in [1u16, 2, 7, 100, 320, 0x8000, 0xffff] {
            for _ in 0..500 {
                assert!(
                    prng.next(modulus) < modulus,
                    "value out of range for {modulus}"
                );
            }
        }
        // modulus 0 returns the raw 16-bit word (no range reduction path).
        let _ = prng.next(0);
    }

    #[test]
    fn randomize_point_cloud_fills_all_records_and_consumes_three_rng_calls_each() {
        let mut prng = BloodPrng::default();
        let points = randomize_ship_3d_point_cloud(&mut prng);
        assert_eq!(points.len(), SHIP_3D_POINT_CLOUD_LEN);
        // Each x/y/z came from next(0xffff), so all are strictly below 0xffff.
        for point in &points {
            assert!(point.x < 0xffff && point.y < 0xffff && point.z < 0xffff);
        }
        // 3 * 0x3E8 = 3000 rng calls advanced the counter (3000 mod 256 = 184).
        assert_eq!(
            prng.counter,
            (3 * SHIP_3D_POINT_CLOUD_LEN as u32 % 256) as u8
        );
        // Not a degenerate all-zero fill.
        assert!(points.iter().any(|p| p.x != 0 || p.y != 0 || p.z != 0));
    }
}
