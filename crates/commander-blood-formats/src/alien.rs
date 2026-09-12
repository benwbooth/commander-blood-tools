//! Typed decoder for the AMER, CROOLIS, and SCRUT interactive 3D overlays.
//!
//! Each XDB stores several paragraph-aligned sections and uses 16-bit offsets
//! within those sections. This loader validates that disk representation once
//! and returns owned models with local parent and vertex indices. Relocation
//! values and original pointer fields are not retained in [`AlienAsset`].

use std::collections::HashMap;

/// Number of spatial components and transform rows.
pub const AXIS_COUNT: usize = 3;
/// Number of entries in the overlays' fixed-point trigonometry table.
pub const TRIGONOMETRY_ENTRY_COUNT: usize = 1_024;
/// Number of entries in the face-raster reciprocal table.
pub const RASTER_RECIPROCAL_COUNT: usize = 500;
/// Width of the indexed texture atlas.
pub const TEXTURE_WIDTH: usize = 256;
/// Height of the indexed texture atlas across its two original banks.
pub const TEXTURE_HEIGHT: usize = 512;
/// Number of entries in the texture-index remap table.
pub const PALETTE_REMAP_ENTRY_COUNT: usize = 256;
/// Number of entries in the scene-wide alien motion-history ring.
pub const ALIEN_RING_ENTRY_COUNT: usize = 128;
/// Number of typed node identities in the alien resume queue.
pub const ALIEN_RESUME_QUEUE_CAPACITY: usize = 8;

const AMER_DATA_DELTA_FIELD: usize = 0x3275;
const BBB_AMER_DATA_DELTA_FIELD: usize = 0x32c5;
const CROOLIS_DATA_DELTA_FIELD: usize = 0x32e5;
const SCRUT_DATA_DELTA_FIELD: usize = 0x33a5;
const AMER_PALETTE_REMAP_POSITION: usize = 0x049b;
const BBB_AMER_PALETTE_REMAP_POSITION: usize = 0x0498;
const OTHER_PALETTE_REMAP_POSITION: usize = 0x04dc;
const PARAGRAPH_BYTE_COUNT: usize = 16;
const DIRECTORY_OBJECT_DELTA_FIELD: usize = 0x000c;
const DIRECTORY_TEXTURE_DELTA_FIELD: usize = 0x000e;
const DIRECTORY_RASTER_DELTA_FIELD: usize = 0x0010;
const TRIGONOMETRY_POSITION: usize = 0x0036;
const TRIGONOMETRY_RECORD_SIZE: usize = 4;
const DISPLAY_PALETTE_POSITION: usize = 0x1f6a;
const INITIAL_METHOD_DELTA_POSITION: usize = 0x0099;
const PALETTE_PREVIOUS_LEVEL_POSITION: usize = 0x009b;
const PALETTE_CYCLE_POSITION: usize = 0x009f;
const PALETTE_CYCLE_STEP_FIELD: usize = 0;
const PALETTE_CYCLE_COUNTDOWN_FIELD: usize = 1;
const OTHER_SCENE_FLAGS_POSITION: usize = 0x02fc;
const PALETTE_PULSE_POSITIONS: [usize; AXIS_COUNT] = [0x2536, 0x2594, 0x25f2];
const PALETTE_ENTRY_COUNT: usize = 256;
const RGB_COMPONENT_COUNT: usize = 3;
const PALETTE_BYTE_COUNT: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
const VGA_DAC_CHANNEL_MAXIMUM: u16 = 63;
const EIGHT_BIT_CHANNEL_MAXIMUM: u16 = 255;
const METHOD_TABLE_POSITION: usize = 0x103a;
const PRIMARY_CONTEXT_POSITION: usize = 0x2306;
const CONTEXT_LIST_POSITION: usize = 0x2308;
const CONTEXT_LIST_LIMIT: usize = 256;
const SCENE_CAMERA_TRANSFORM_POSITION: usize = 0x22a8;
const CAMERA_MATRIX_POSITION: usize = 0x22ba;
const CAMERA_RESULT_POSITION: usize = 0x22de;
const CAMERA_POSITION_POSITION: usize = 0x22ea;
const CAMERA_ANGLE_POSITION: usize = 0x22f6;
const CAMERA_DEPTH_VELOCITY_POSITION: usize = 0x22fc;
const CAMERA_HORIZONTAL_FILTER_POSITION: usize = 0x1058;
const BEHAVIOR_RANDOM_STATE_POSITION: usize = 0x105c;
const AMER_STAR_SHADE_TABLE_POSITION: usize = 0x07d4;
const OTHER_STAR_SHADE_TABLE_POSITION: usize = 0x07d6;
const AMER_STAR_SEED_POSITION: usize = 0x08d4;
const OTHER_STAR_SEED_POSITION: usize = 0x08d6;
const STAR_SHADE_TABLE_ENTRY_COUNT: usize = 256;
const MODEL_MAGIC_POSITION: usize = 0x0000;
const MODEL_MAGIC: &[u8; 4] = b"3DB0";
const MODEL_HEADER_SIZE_FIELD: usize = 0x0004;
const MODEL_HEADER_SIZE: u16 = 0x0048;
const MODEL_VERSION_FIELD: usize = 0x0006;
const MODEL_VERSION: [u8; 2] = [1, 2];
const MODEL_NAME_FIELD: usize = 0x0008;
const MODEL_NAME_LENGTH: usize = 8;
const MODEL_ROOT_FIELD: usize = 0x0016;
const MODEL_NODE_COUNT_FIELD: usize = 0x001a;
const PRIMARY_VERTEX_START_FIELD: usize = 0x001c;
const PRIMARY_VERTEX_COUNT_FIELD: usize = 0x0020;
const MODEL_COPY_START_FIELD: usize = 0x0022;
const MODEL_COPY_COUNT_FIELD: usize = 0x0026;
const MODEL_FACE_START_FIELD: usize = 0x0028;
const MODEL_FACE_COUNT_FIELD: usize = 0x002c;
const MODEL_METHOD_TABLE_OFFSET_FIELD: usize = 0x0034;
const TRANSFORM_RECORD_SIZE: usize = 0x005e;
const NODE_PARENT_FIELD: usize = 0x0000;
const NODE_VERTEX_COUNT_FIELD: usize = 0x0002;
const NODE_VERTEX_START_FIELD: usize = 0x0006;
const NODE_MATRIX_FIELD: usize = 0x0012;
const NODE_TRANSLATION_FIELD: usize = 0x0036;
const NODE_LOCAL_POSITION_FIELD: usize = 0x0042;
const NODE_ANGLE_FIELD: usize = 0x004e;
const NODE_RADIAL_OFFSET_FIELD: usize = 0x0054;
const VERTEX_RECORD_SIZE: usize = 20;
const VERTEX_TEXTURE_FIELD: usize = 0x0000;
const VERTEX_POSITION_FIELD: usize = 0x0004;
const VERTEX_SCREEN_FIELD: usize = 0x000a;
const VERTEX_RASTER_DEPTH_FIELD: usize = 0x000e;
const FACE_RECORD_SIZE: usize = 8;
const FACE_FIRST_VERTEX_FIELD: usize = 0x0002;
const METHOD_SLOT_SIZE: usize = 2;
const METHOD_SLOT_NOOP_PRIMARY: usize = 0;
const METHOD_SLOT_WAVE: usize = 1;
const METHOD_SLOT_DISPATCH_PRIMARY: usize = 2;
const METHOD_SLOT_RING: usize = 3;
const METHOD_SLOT_DISPATCH_SECONDARY: usize = 4;
const METHOD_SLOT_NOOP_SECONDARY: usize = 5;
const METHOD_SLOT_WRAP_POSITIONS: usize = 6;
const METHOD_SLOT_PALETTE: usize = 7;
const METHOD_SLOT_SAMPLE_DELTA: usize = 8;
const METHOD_SLOT_SCALED_SAMPLE_DELTA: usize = 9;
const METHOD_SLOT_BOUNDS_WRAP: usize = 10;
const METHOD_SLOT_ANCHOR: usize = 11;
const METHOD_SLOT_ADJUST_STATE: usize = 12;
const METHOD_SLOT_RESUME: usize = 13;
const METHOD_SLOT_NOOP_TERTIARY: usize = 14;
const METHOD_CONTROL_FIELD: usize = 0x0036;
const METHOD_CONTINUATION_FIELD: usize = 0x0038;
const WAVE_PRIMARY_PHASE_FIELD: usize = METHOD_CONTINUATION_FIELD;
const WAVE_PRIMARY_STEP_FIELD: usize = METHOD_CONTINUATION_FIELD + 2;
const WAVE_SECONDARY_PHASE_FIELD: usize = METHOD_CONTINUATION_FIELD + 4;
const WAVE_SECONDARY_STEP_FIELD: usize = METHOD_CONTINUATION_FIELD + 6;
const AMER_WAVE_SCENE_STATE_POSITION: usize = 0x0b2f;
const BBB_AMER_WAVE_SCENE_STATE_POSITION: usize = 0x0b2c;
const OTHER_WAVE_SCENE_STATE_POSITION: usize = 0x0b70;
const WAVE_SELECTED_NODE_FIELD: usize = 4;
const WAVE_CURRENT_SAMPLE_FIELD: usize = 6;
const RING_NODE_CALLBACK_FIELD: usize = 0x000e;
const RING_NODE_COURSE_FRAMES_FIELD: usize = 0x0056;
const RING_NODE_FEEDBACK_PHASE_FIELD: usize = 0x0058;
const RING_NODE_CURSOR_FIELD: usize = 0x005a;
const RING_NODE_BEHAVIOR_SEED_FIELD: usize = 0x005c;
const SLOT2_NODE_CALLBACK_FIELD: usize = RING_NODE_CALLBACK_FIELD;
const SLOT2_NODE_MOTION_PARAMETER_FIELD: usize = RING_NODE_COURSE_FRAMES_FIELD;
const SLOT2_NODE_RADIAL_TARGET_FIELD: usize = RING_NODE_FEEDBACK_PHASE_FIELD;
const SLOT2_NODE_SECONDARY_MOTION_FIELD: usize = RING_NODE_CURSOR_FIELD;
const SLOT2_NODE_BEHAVIOR_SEED_FIELD: usize = RING_NODE_BEHAVIOR_SEED_FIELD;
const SLOT2_PHASE_TIMER_FIELD: usize = 0x0038;
const SLOT2_SECONDARY_CONTEXT_FIELD: usize = 0x003a;
const SLOT2_TERTIARY_CONTEXT_FIELD: usize = 0x003c;
const SLOT2_AMER_RANDOM_FIELD: usize = 0x0040;
const SLOT2_COMMON_RANDOM_FIELD: usize = 0x0042;
const SLOT2_AMER_ANIMATION_PHASE_FIELD: usize = 0x0042;
const RESUME_PHASE_FIELD: usize = METHOD_CONTINUATION_FIELD;
const RESUME_PAIRED_NODE_FIELD: usize = METHOD_CONTINUATION_FIELD + 2;
const RESUME_RESUMED_NODE_FIELD: usize = METHOD_CONTINUATION_FIELD + 4;
const RING_ENTRY_SIZE: usize = 8;
const RING_ENTRY_PITCH_STEP_FIELD: usize = 0;
const RING_ENTRY_PAN_STEP_FIELD: usize = 2;
const RING_ENTRY_RADIAL_OFFSET_FIELD: usize = 4;
const RING_ENTRY_COMMAND_FLAGS_FIELD: usize = 6;
const AMER_RING_TIMER_POSITION: usize = 0x0b31;
const BBB_AMER_RING_TIMER_POSITION: usize = 0x0b2e;
const CROOLIS_RING_TIMER_POSITION: usize = 0x0b72;
const SCRUT_RING_TIMER_POSITION: usize = 0x0b72;
const AMER_RING_GENERATION_POSITION: usize = 0x0d5b;
const BBB_AMER_RING_GENERATION_POSITION: usize = 0x0d6a;
const CROOLIS_RING_GENERATION_POSITION: usize = 0x0db3;
const SCRUT_RING_GENERATION_POSITION: usize = 0x0da1;
const AMER_RING_CURSOR_POSITION: usize = 0x0d5d;
const BBB_AMER_RING_CURSOR_POSITION: usize = 0x0d6c;
const CROOLIS_RING_CURSOR_POSITION: usize = 0x0db5;
const SCRUT_RING_CURSOR_POSITION: usize = 0x0da3;
const AMER_RING_RESUME_COUNTDOWN_POSITION: usize = 0x0d5f;
const BBB_AMER_RING_RESUME_COUNTDOWN_POSITION: usize = 0x0d6e;
const CROOLIS_RING_RESUME_COUNTDOWN_POSITION: usize = 0x0db7;
const SCRUT_RING_RESUME_COUNTDOWN_POSITION: usize = 0x0da5;
const AMER_RING_RESUME_NODE_POSITION: usize = 0x0d61;
const BBB_AMER_RING_RESUME_NODE_POSITION: usize = 0x0d70;
const CROOLIS_RING_RESUME_NODE_POSITION: usize = 0x0db9;
const SCRUT_RING_RESUME_NODE_POSITION: usize = 0x0da7;
const AMER_RING_ENTRIES_POSITION: usize = 0x0d63;
const BBB_AMER_RING_ENTRIES_POSITION: usize = 0x0d72;
const CROOLIS_RING_ENTRIES_POSITION: usize = 0x0dbb;
const SCRUT_RING_ENTRIES_POSITION: usize = 0x0da9;
const AMER_INITIAL_COURSE_CALLBACK: u16 = 0x12b3;
const BBB_AMER_INITIAL_COURSE_CALLBACK: u16 = 0x12c2;
const CROOLIS_INITIAL_COURSE_CALLBACK: u16 = 0x130b;
const SCRUT_INITIAL_COURSE_CALLBACK: u16 = 0x12f9;
const AMER_FOLLOW_COURSE_CALLBACK: u16 = 0x1414;
const BBB_AMER_FOLLOW_COURSE_CALLBACK: u16 = 0x1423;
const CROOLIS_FOLLOW_COURSE_CALLBACK: u16 = 0x146c;
const SCRUT_FOLLOW_COURSE_CALLBACK: u16 = 0x145a;
const AMER_SLOT2_UPDATE_CALLBACK: u16 = 0x1692;
const AMER_SLOT2_STEER_CALLBACK: u16 = 0x1a5c;
const AMER_SLOT2_FINISH_CALLBACK: u16 = 0x1aa0;
const BBB_AMER_SLOT2_UPDATE_CALLBACK: u16 = 0x16a1;
const BBB_AMER_SLOT2_STEER_CALLBACK: u16 = 0x1a62;
const BBB_AMER_SLOT2_FINISH_CALLBACK: u16 = 0x1aa6;
const CROOLIS_SLOT2_UPDATE_CALLBACK: u16 = 0x1727;
const SCRUT_SLOT2_UPDATE_CALLBACK: u16 = 0x171b;
const AMER_SLOT2_ACTIVE_POSITION: usize = 0x1648;
const BBB_AMER_SLOT2_ACTIVE_POSITION: usize = 0x1657;
const CROOLIS_SLOT2_ACTIVE_POSITION: usize = 0x16a0;
const SCRUT_SLOT2_ACTIVE_POSITION: usize = 0x168e;
const CROOLIS_SLOT2_SPECIES_SEED_POSITION: usize = 0x16a2;
const SCRUT_SLOT2_SPECIES_SEED_POSITION: usize = 0x1690;
const AMER_RESUME_ANCHOR_POSITION: usize = 0x1bc2;
const BBB_AMER_RESUME_ANCHOR_POSITION: usize = 0x1c12;
const AMER_RESUME_CURRENT_POSITION: usize = 0x1bc4;
const BBB_AMER_RESUME_CURRENT_POSITION: usize = 0x1c14;
const AMER_RESUME_WRITE_CURSOR_POSITION: usize = 0x1bc6;
const BBB_AMER_RESUME_WRITE_CURSOR_POSITION: usize = 0x1c16;
const AMER_RESUME_READ_CURSOR_POSITION: usize = 0x1bc8;
const BBB_AMER_RESUME_READ_CURSOR_POSITION: usize = 0x1c18;
const AMER_RESUME_QUEUE_POSITION: usize = 0x1bca;
const BBB_AMER_RESUME_QUEUE_POSITION: usize = 0x1c1a;
const CROOLIS_RESUME_ANCHOR_POSITION: usize = 0x1b2e;
const CROOLIS_RESUME_CURRENT_POSITION: usize = 0x1b30;
const CROOLIS_RESUME_WRITE_CURSOR_POSITION: usize = 0x1b32;
const CROOLIS_RESUME_READ_CURSOR_POSITION: usize = 0x1b34;
const CROOLIS_RESUME_QUEUE_POSITION: usize = 0x1b36;
const BBB_CROOLIS_RESUME_ANCHOR_POSITION: usize = 0x1b34;
const BBB_CROOLIS_RESUME_CURRENT_POSITION: usize = 0x1b36;
const BBB_CROOLIS_RESUME_WRITE_CURSOR_POSITION: usize = 0x1b38;
const BBB_CROOLIS_RESUME_READ_CURSOR_POSITION: usize = 0x1b3a;
const BBB_CROOLIS_RESUME_QUEUE_POSITION: usize = 0x1b3c;
const SCRUT_RESUME_ANCHOR_POSITION: usize = 0x1be3;
const SCRUT_RESUME_CURRENT_POSITION: usize = 0x1be5;
const SCRUT_RESUME_WRITE_CURSOR_POSITION: usize = 0x1be7;
const SCRUT_RESUME_READ_CURSOR_POSITION: usize = 0x1be9;
const SCRUT_RESUME_QUEUE_POSITION: usize = 0x1beb;
const AMER_RESUME_BEGIN_CALLBACK: u16 = 0x1c34;
const BBB_AMER_RESUME_BEGIN_CALLBACK: u16 = 0x1c84;
const AMER_RESUME_PAIR_CALLBACK: u16 = 0x1c7d;
const BBB_AMER_RESUME_PAIR_CALLBACK: u16 = 0x1ccd;
const AMER_RESUME_TIMEOUT_CALLBACK: u16 = 0x1cbf;
const BBB_AMER_RESUME_TIMEOUT_CALLBACK: u16 = 0x1d0f;
const AMER_RESUME_FINAL_CALLBACK: u16 = 0x1ccf;
const BBB_AMER_RESUME_FINAL_CALLBACK: u16 = 0x1d1f;
const CROOLIS_RESUME_BEGIN_CALLBACK: u16 = 0x1b85;
const CROOLIS_RESUME_PAIR_CALLBACK: u16 = 0x1bc9;
const CROOLIS_RESUME_TIMEOUT_CALLBACK: u16 = 0x1c0b;
const CROOLIS_RESUME_FINAL_CALLBACK: u16 = 0x1c1b;
const BBB_CROOLIS_RESUME_BEGIN_CALLBACK: u16 = 0x1b8b;
const BBB_CROOLIS_RESUME_PAIR_CALLBACK: u16 = 0x1bcf;
const BBB_CROOLIS_RESUME_TIMEOUT_CALLBACK: u16 = 0x1c11;
const BBB_CROOLIS_RESUME_FINAL_CALLBACK: u16 = 0x1c21;
const SCRUT_RESUME_BEGIN_CALLBACK: u16 = 0x1c45;
const SCRUT_RESUME_PAIR_CALLBACK: u16 = 0x1c89;
const SCRUT_RESUME_TIMEOUT_CALLBACK: u16 = 0x1ccb;
const SCRUT_RESUME_FINAL_CALLBACK: u16 = 0x1cdb;
// All shipped overlays initialize this write-only continuation field identically.
const SHIPPED_UNUSED_RESUMED_NODE_WORD: u16 = 0x1da4;
const RESUME_QUEUE_ENTRY_SIZE: usize = 2;
const INVALID_METHOD_ENTRY: u16 = 0xffff;
const ZERO_COORDINATE: i16 = 0;
const ZERO_POSITION: [i16; AXIS_COUNT] = [ZERO_COORDINATE; AXIS_COUNT];
const METHOD_ENTRY_COUNT: usize = 15;
const COMMANDER_AMER_METHOD_TABLE: [u16; METHOD_ENTRY_COUNT] = [
    0x1dd6, 0x09ef, 0x164c, 0x1286, 0x1dd6, 0x1dd6, 0x0958, 0x0355, 0x1b5f, 0x1b8f, 0x0925, 0x0b0f,
    0x0b1f, 0x1bea, 0x1dd6,
];
const BBB_AMER_METHOD_TABLE: [u16; METHOD_ENTRY_COUNT] = [
    0x1e24, 0x09ec, 0x165b, 0x1295, 0x1e24, 0x1e24, 0x0955, 0x0352, 0x1baf, 0x1bdf, 0x0922, 0x0b0c,
    0x0b1c, 0x1c3a, 0x1e24,
];
const COMMANDER_CROOLIS_METHOD_TABLE: [u16; METHOD_ENTRY_COUNT] = [
    0x1d27, 0x0a30, 0x16a4, 0x12de, 0x16a4, 0x1d27, 0x0999, 0x036a, 0x1acb, 0x1afb, 0x0966, 0x0b50,
    0x0b60, 0x1b46, 0x1d27,
];
const BBB_CROOLIS_METHOD_TABLE: [u16; METHOD_ENTRY_COUNT] = [
    0x1d29, 0x0a30, 0x16a4, 0x12de, 0x16a4, 0x1d29, 0x0999, 0x036a, 0x1ad1, 0x1b01, 0x0966, 0x0b50,
    0x0b60, 0x1b4c, 0x1d29,
];
const COMMANDER_SCRUT_METHOD_TABLE: [u16; METHOD_ENTRY_COUNT] = [
    0x1de7, 0x0a35, 0x1692, 0x12cc, 0x1692, 0x1de7, 0x0999, 0x036a, 0x1b80, 0x1bb0, 0x0966, 0x0b55,
    0x0b65, 0x1bfb, 0x1de7,
];

/// Alien overlay format variant being decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienXdbKind {
    /// `AMER.XDB`.
    Amer,
    /// `CROOLIS.XDB`.
    Croolis,
    /// `SCRUT.XDB`.
    Scrut,
}

/// Original game revision that supplied an alien overlay image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienXdbRevision {
    /// Commander Blood 2.0 overlay.
    CommanderBlood,
    /// Big Bug Bang overlay.
    BigBugBang,
}

impl AlienXdbKind {
    fn star_shade_table_position(self) -> usize {
        match self {
            Self::Amer => AMER_STAR_SHADE_TABLE_POSITION,
            Self::Croolis | Self::Scrut => OTHER_STAR_SHADE_TABLE_POSITION,
        }
    }

    fn star_seed_position(self) -> usize {
        match self {
            Self::Amer => AMER_STAR_SEED_POSITION,
            Self::Croolis | Self::Scrut => OTHER_STAR_SEED_POSITION,
        }
    }
}

/// Decoder-only locations and callback values in one original XDB image.
#[derive(Clone, Copy)]
struct AlienRingSourceLayout {
    timer_position: usize,
    generation_position: usize,
    cursor_position: usize,
    resume_countdown_position: usize,
    resume_node_position: usize,
    entries_position: usize,
    initial_course_callback: u16,
    follow_course_callback: u16,
}

/// Decoder-only source locations and callback values for resume state.
#[derive(Clone, Copy)]
struct AlienResumeSourceLayout {
    anchor_position: usize,
    current_position: usize,
    write_cursor_position: usize,
    read_cursor_position: usize,
    queue_position: usize,
    begin_callback: u16,
    pair_callback: u16,
    timeout_callback: u16,
    final_callback: u16,
}

#[derive(Clone, Copy)]
struct AlienSlot2SourceLayout {
    active_position: usize,
    species_seed_position: Option<usize>,
    update_callback: u16,
    amer_steer_callback: Option<u16>,
    amer_finish_callback: Option<u16>,
}

#[derive(Clone, Copy)]
struct AlienXdbSourceLayout {
    revision: AlienXdbRevision,
    data_delta_field: usize,
    palette_remap_position: usize,
    wave_scene_state_position: usize,
    ring: AlienRingSourceLayout,
    slot2: AlienSlot2SourceLayout,
    resume: AlienResumeSourceLayout,
    method_table: &'static [u16; METHOD_ENTRY_COUNT],
}

fn commander_source_layout(kind: AlienXdbKind) -> AlienXdbSourceLayout {
    match kind {
        AlienXdbKind::Amer => AlienXdbSourceLayout {
            revision: AlienXdbRevision::CommanderBlood,
            data_delta_field: AMER_DATA_DELTA_FIELD,
            palette_remap_position: AMER_PALETTE_REMAP_POSITION,
            wave_scene_state_position: AMER_WAVE_SCENE_STATE_POSITION,
            ring: AlienRingSourceLayout {
                timer_position: AMER_RING_TIMER_POSITION,
                generation_position: AMER_RING_GENERATION_POSITION,
                cursor_position: AMER_RING_CURSOR_POSITION,
                resume_countdown_position: AMER_RING_RESUME_COUNTDOWN_POSITION,
                resume_node_position: AMER_RING_RESUME_NODE_POSITION,
                entries_position: AMER_RING_ENTRIES_POSITION,
                initial_course_callback: AMER_INITIAL_COURSE_CALLBACK,
                follow_course_callback: AMER_FOLLOW_COURSE_CALLBACK,
            },
            slot2: AlienSlot2SourceLayout {
                active_position: AMER_SLOT2_ACTIVE_POSITION,
                species_seed_position: None,
                update_callback: AMER_SLOT2_UPDATE_CALLBACK,
                amer_steer_callback: Some(AMER_SLOT2_STEER_CALLBACK),
                amer_finish_callback: Some(AMER_SLOT2_FINISH_CALLBACK),
            },
            resume: AlienResumeSourceLayout {
                anchor_position: AMER_RESUME_ANCHOR_POSITION,
                current_position: AMER_RESUME_CURRENT_POSITION,
                write_cursor_position: AMER_RESUME_WRITE_CURSOR_POSITION,
                read_cursor_position: AMER_RESUME_READ_CURSOR_POSITION,
                queue_position: AMER_RESUME_QUEUE_POSITION,
                begin_callback: AMER_RESUME_BEGIN_CALLBACK,
                pair_callback: AMER_RESUME_PAIR_CALLBACK,
                timeout_callback: AMER_RESUME_TIMEOUT_CALLBACK,
                final_callback: AMER_RESUME_FINAL_CALLBACK,
            },
            method_table: &COMMANDER_AMER_METHOD_TABLE,
        },
        AlienXdbKind::Croolis => AlienXdbSourceLayout {
            revision: AlienXdbRevision::CommanderBlood,
            data_delta_field: CROOLIS_DATA_DELTA_FIELD,
            palette_remap_position: OTHER_PALETTE_REMAP_POSITION,
            wave_scene_state_position: OTHER_WAVE_SCENE_STATE_POSITION,
            ring: AlienRingSourceLayout {
                timer_position: CROOLIS_RING_TIMER_POSITION,
                generation_position: CROOLIS_RING_GENERATION_POSITION,
                cursor_position: CROOLIS_RING_CURSOR_POSITION,
                resume_countdown_position: CROOLIS_RING_RESUME_COUNTDOWN_POSITION,
                resume_node_position: CROOLIS_RING_RESUME_NODE_POSITION,
                entries_position: CROOLIS_RING_ENTRIES_POSITION,
                initial_course_callback: CROOLIS_INITIAL_COURSE_CALLBACK,
                follow_course_callback: CROOLIS_FOLLOW_COURSE_CALLBACK,
            },
            slot2: AlienSlot2SourceLayout {
                active_position: CROOLIS_SLOT2_ACTIVE_POSITION,
                species_seed_position: Some(CROOLIS_SLOT2_SPECIES_SEED_POSITION),
                update_callback: CROOLIS_SLOT2_UPDATE_CALLBACK,
                amer_steer_callback: None,
                amer_finish_callback: None,
            },
            resume: AlienResumeSourceLayout {
                anchor_position: CROOLIS_RESUME_ANCHOR_POSITION,
                current_position: CROOLIS_RESUME_CURRENT_POSITION,
                write_cursor_position: CROOLIS_RESUME_WRITE_CURSOR_POSITION,
                read_cursor_position: CROOLIS_RESUME_READ_CURSOR_POSITION,
                queue_position: CROOLIS_RESUME_QUEUE_POSITION,
                begin_callback: CROOLIS_RESUME_BEGIN_CALLBACK,
                pair_callback: CROOLIS_RESUME_PAIR_CALLBACK,
                timeout_callback: CROOLIS_RESUME_TIMEOUT_CALLBACK,
                final_callback: CROOLIS_RESUME_FINAL_CALLBACK,
            },
            method_table: &COMMANDER_CROOLIS_METHOD_TABLE,
        },
        AlienXdbKind::Scrut => AlienXdbSourceLayout {
            revision: AlienXdbRevision::CommanderBlood,
            data_delta_field: SCRUT_DATA_DELTA_FIELD,
            palette_remap_position: OTHER_PALETTE_REMAP_POSITION,
            wave_scene_state_position: OTHER_WAVE_SCENE_STATE_POSITION,
            ring: AlienRingSourceLayout {
                timer_position: SCRUT_RING_TIMER_POSITION,
                generation_position: SCRUT_RING_GENERATION_POSITION,
                cursor_position: SCRUT_RING_CURSOR_POSITION,
                resume_countdown_position: SCRUT_RING_RESUME_COUNTDOWN_POSITION,
                resume_node_position: SCRUT_RING_RESUME_NODE_POSITION,
                entries_position: SCRUT_RING_ENTRIES_POSITION,
                initial_course_callback: SCRUT_INITIAL_COURSE_CALLBACK,
                follow_course_callback: SCRUT_FOLLOW_COURSE_CALLBACK,
            },
            slot2: AlienSlot2SourceLayout {
                active_position: SCRUT_SLOT2_ACTIVE_POSITION,
                species_seed_position: Some(SCRUT_SLOT2_SPECIES_SEED_POSITION),
                update_callback: SCRUT_SLOT2_UPDATE_CALLBACK,
                amer_steer_callback: None,
                amer_finish_callback: None,
            },
            resume: AlienResumeSourceLayout {
                anchor_position: SCRUT_RESUME_ANCHOR_POSITION,
                current_position: SCRUT_RESUME_CURRENT_POSITION,
                write_cursor_position: SCRUT_RESUME_WRITE_CURSOR_POSITION,
                read_cursor_position: SCRUT_RESUME_READ_CURSOR_POSITION,
                queue_position: SCRUT_RESUME_QUEUE_POSITION,
                begin_callback: SCRUT_RESUME_BEGIN_CALLBACK,
                pair_callback: SCRUT_RESUME_PAIR_CALLBACK,
                timeout_callback: SCRUT_RESUME_TIMEOUT_CALLBACK,
                final_callback: SCRUT_RESUME_FINAL_CALLBACK,
            },
            method_table: &COMMANDER_SCRUT_METHOD_TABLE,
        },
    }
}

fn big_bug_bang_source_layout(kind: AlienXdbKind) -> Option<AlienXdbSourceLayout> {
    Some(match kind {
        AlienXdbKind::Amer => AlienXdbSourceLayout {
            revision: AlienXdbRevision::BigBugBang,
            data_delta_field: BBB_AMER_DATA_DELTA_FIELD,
            palette_remap_position: BBB_AMER_PALETTE_REMAP_POSITION,
            wave_scene_state_position: BBB_AMER_WAVE_SCENE_STATE_POSITION,
            ring: AlienRingSourceLayout {
                timer_position: BBB_AMER_RING_TIMER_POSITION,
                generation_position: BBB_AMER_RING_GENERATION_POSITION,
                cursor_position: BBB_AMER_RING_CURSOR_POSITION,
                resume_countdown_position: BBB_AMER_RING_RESUME_COUNTDOWN_POSITION,
                resume_node_position: BBB_AMER_RING_RESUME_NODE_POSITION,
                entries_position: BBB_AMER_RING_ENTRIES_POSITION,
                initial_course_callback: BBB_AMER_INITIAL_COURSE_CALLBACK,
                follow_course_callback: BBB_AMER_FOLLOW_COURSE_CALLBACK,
            },
            slot2: AlienSlot2SourceLayout {
                active_position: BBB_AMER_SLOT2_ACTIVE_POSITION,
                species_seed_position: None,
                update_callback: BBB_AMER_SLOT2_UPDATE_CALLBACK,
                amer_steer_callback: Some(BBB_AMER_SLOT2_STEER_CALLBACK),
                amer_finish_callback: Some(BBB_AMER_SLOT2_FINISH_CALLBACK),
            },
            resume: AlienResumeSourceLayout {
                anchor_position: BBB_AMER_RESUME_ANCHOR_POSITION,
                current_position: BBB_AMER_RESUME_CURRENT_POSITION,
                write_cursor_position: BBB_AMER_RESUME_WRITE_CURSOR_POSITION,
                read_cursor_position: BBB_AMER_RESUME_READ_CURSOR_POSITION,
                queue_position: BBB_AMER_RESUME_QUEUE_POSITION,
                begin_callback: BBB_AMER_RESUME_BEGIN_CALLBACK,
                pair_callback: BBB_AMER_RESUME_PAIR_CALLBACK,
                timeout_callback: BBB_AMER_RESUME_TIMEOUT_CALLBACK,
                final_callback: BBB_AMER_RESUME_FINAL_CALLBACK,
            },
            method_table: &BBB_AMER_METHOD_TABLE,
        },
        AlienXdbKind::Croolis => AlienXdbSourceLayout {
            revision: AlienXdbRevision::BigBugBang,
            data_delta_field: CROOLIS_DATA_DELTA_FIELD,
            palette_remap_position: OTHER_PALETTE_REMAP_POSITION,
            wave_scene_state_position: OTHER_WAVE_SCENE_STATE_POSITION,
            ring: commander_source_layout(kind).ring,
            slot2: commander_source_layout(kind).slot2,
            resume: AlienResumeSourceLayout {
                anchor_position: BBB_CROOLIS_RESUME_ANCHOR_POSITION,
                current_position: BBB_CROOLIS_RESUME_CURRENT_POSITION,
                write_cursor_position: BBB_CROOLIS_RESUME_WRITE_CURSOR_POSITION,
                read_cursor_position: BBB_CROOLIS_RESUME_READ_CURSOR_POSITION,
                queue_position: BBB_CROOLIS_RESUME_QUEUE_POSITION,
                begin_callback: BBB_CROOLIS_RESUME_BEGIN_CALLBACK,
                pair_callback: BBB_CROOLIS_RESUME_PAIR_CALLBACK,
                timeout_callback: BBB_CROOLIS_RESUME_TIMEOUT_CALLBACK,
                final_callback: BBB_CROOLIS_RESUME_FINAL_CALLBACK,
            },
            method_table: &BBB_CROOLIS_METHOD_TABLE,
        },
        AlienXdbKind::Scrut => return None,
    })
}

fn method_table_matches(
    data: &[u8],
    data_start: usize,
    expected: &[u16; METHOD_ENTRY_COUNT],
) -> bool {
    expected.iter().copied().enumerate().all(|(index, value)| {
        read_u16(
            data,
            data_start + METHOD_TABLE_POSITION + index * METHOD_SLOT_SIZE,
        ) == Some(value)
    })
}

fn source_layout(data: &[u8], kind: AlienXdbKind) -> Option<(AlienXdbSourceLayout, usize)> {
    let candidates = [
        Some(commander_source_layout(kind)),
        big_bug_bang_source_layout(kind),
    ];
    candidates.into_iter().flatten().find_map(|layout| {
        let data_start = usize::from(read_u16(data, layout.data_delta_field)?)
            .checked_mul(PARAGRAPH_BYTE_COUNT)?;
        method_table_matches(data, data_start, layout.method_table).then_some((layout, data_start))
    })
}

/// Fixed-point cosine and sine pair from an alien XDB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienTrigonometryPair {
    /// Cosine component.
    pub cosine: i16,
    /// Sine component.
    pub sine: i16,
}

/// Initial row-major transform stored in a model node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienTransformData {
    /// Three-by-three orientation matrix.
    pub matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Three-component translation.
    pub translation: [i32; AXIS_COUNT],
}

/// Initial camera and control values authored in one alien overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCameraData {
    /// Initial eased camera orientation matrix.
    pub matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Initial fixed-point camera position.
    pub position: [i32; AXIS_COUNT],
    /// Initial matrix-transformed view vector.
    pub transformed_view: [i32; AXIS_COUNT],
    /// Initial pitch, pan, and secondary-pan accumulators.
    pub angles: [i16; AXIS_COUNT],
    /// Initial forward/backward camera velocity.
    pub depth_velocity: i16,
    /// Initial horizontal mouse filter accumulator.
    pub horizontal_filter: i16,
}

/// Typed parent relation in one model hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienNodeParent {
    /// Shared camera transform synthesized by the scene controller.
    SceneCamera,
    /// Model root transform.
    Root,
    /// Earlier node in the same model.
    Node(usize),
}

/// Authored hierarchical transform and its consecutive vertex range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienNodeData {
    /// Root or earlier node supplying the parent transform.
    pub parent: AlienNodeParent,
    /// First vertex controlled by this node.
    pub first_vertex: usize,
    /// Number of consecutive vertices controlled by this node.
    pub vertex_count: usize,
    /// Initial composed transform.
    pub transform: AlienTransformData,
    /// Mutable local-position accumulators.
    pub local_position: [i32; AXIS_COUNT],
    /// Wrapping Euler angles.
    pub angles: [u16; AXIS_COUNT],
    /// Signed radial displacement.
    pub radial_offset: i16,
}

/// Texture and object-space coordinates for one mesh vertex.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienVertexData {
    /// Texture coordinates in the 256-by-512 atlas.
    pub texture: [i16; 2],
    /// Object-space position; an alias retains zero and uses a projection copy.
    pub position: [i16; AXIS_COUNT],
    /// Initial projected coordinate retained by primary vertices with invalid depth.
    pub initial_screen: [i16; 2],
    /// Authored depth/interpolation value consumed by the face raster stage.
    pub raster_depth: i32,
}

/// Projection sharing for a UV-seam alias vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienProjectionCopyData {
    /// Authored vertex supplying projected position and depth.
    pub source: usize,
    /// Alias vertex receiving projection while retaining independent UVs.
    pub destination: usize,
}

/// Three local vertex indices forming one authored triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienFaceData {
    /// Indices into [`AlienMeshData::vertices`].
    pub vertices: [usize; AXIS_COUNT],
}

/// Owned geometry used by either the primary mesh or a behavior model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienMeshData {
    /// Authored vertices followed by projection aliases.
    pub vertices: Vec<AlienVertexData>,
    /// Projection copies for UV aliases.
    pub projection_copies: Vec<AlienProjectionCopyData>,
    /// Triangle list using local vertex indices.
    pub faces: Vec<AlienFaceData>,
}

/// Named camera-relative mesh rendered before the behavior models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienPrimaryModelData {
    /// Eight-character authored model name.
    pub name: String,
    /// Primary model geometry.
    pub mesh: AlienMeshData,
}

/// Semantic method selected by a model's recovered dispatch-table slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienBehaviorMethod {
    /// No per-frame behavior.
    NoOperation,
    /// Proximity-selected wave motion.
    Wave,
    /// Species animation dispatch shared by method slots two and four.
    AnimationDispatch,
    /// Ring-driven animation update.
    RingAnimation,
    /// Camera-relative position wrapping.
    WrapPositions,
    /// Palette pulse and cycle update.
    PaletteUpdate,
    /// Apply an unscaled cyclic sample delta.
    ApplySampleDelta,
    /// Apply a distance-scaled cyclic sample delta.
    ApplyScaledSampleDelta,
    /// Bounds test followed by position wrapping.
    BoundsThenWrap,
    /// Publish the selected anchor state.
    AnchorState,
    /// Apply the species-specific state adjustment.
    AdjustState,
    /// Resume a queued behavior state.
    Resume,
}

/// Authored wave-selection lifecycle stored in an alien overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienWaveSelectionData {
    /// No model-selection check is pending.
    Disabled,
    /// A model-selection check is pending.
    Requested,
    /// One wave model satisfied the camera-relative bounds.
    Selected,
}

/// Flat reference to one decoded model node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienModelNodeReference {
    /// Model in authored context order.
    pub model_index: usize,
    /// Node in the model's hierarchy order.
    pub node_index: usize,
}

/// Initial continuation state stored in one wave-method context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienWaveMethodData {
    /// Whether the original one-time initializer has already run.
    pub initialized: bool,
    /// Primary cyclic sample phase.
    pub primary_phase: u16,
    /// Signed primary phase advance.
    pub primary_step: i16,
    /// Distance-weighted secondary sample phase.
    pub secondary_phase: u16,
    /// Signed secondary phase advance.
    pub secondary_step: i16,
}

/// Initial scene-wide state shared by all wave contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienWaveSceneData {
    /// Initial model-selection lifecycle.
    pub selection: AlienWaveSelectionData,
    /// Initially selected wave node, when authored.
    pub selected_node: Option<AlienModelNodeReference>,
    /// Initial cosine sample published to wave callbacks.
    pub current_sample: i16,
}

/// Initial shared state for the palette-animation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienPaletteAnimationData {
    /// Previous texture-remap level.
    pub previous_level: u16,
    /// Signed texture-remap phase increment.
    pub step: i8,
    /// Frames remaining before reversing the increment.
    pub countdown: u8,
    /// Initial low-word palette pulse levels.
    pub pulse_levels: [u16; AXIS_COUNT],
}

/// Initial timer policy for one authored ring-animation model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienRingLifecycleData {
    /// The model has not run its one-time ring initializer.
    Uninitialized,
    /// The model advances the scene-wide ring timer before dispatching nodes.
    TimerRunning,
    /// The model dispatches nodes without advancing the shared timer.
    TimerSuspended,
}

/// Callback stages that can be present in an unmodified XDB model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienRingInitialCallbackData {
    /// Generate motion history for the leading node.
    InitialCourse,
    /// Consume motion history produced by the leading node.
    FollowCourse,
}

/// Initial semantic state for one node in a ring-animation model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienRingNodeData {
    /// Authored callback stage.
    pub callback: AlienRingInitialCallbackData,
    /// Frames remaining in the current generated course.
    pub course_frames_remaining: i16,
    /// Cyclic phase used by callback feedback.
    pub feedback_phase: u16,
    /// Flat index into the shared motion-history ring.
    pub ring_slot: usize,
    /// Deterministic callback seed or stage marker.
    pub behavior_seed: u16,
}

/// Initial continuation state for one ring-animation model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienRingModelData {
    /// Authored initialization and timer policy.
    pub lifecycle: AlienRingLifecycleData,
    /// Behavior state parallel to the model hierarchy.
    pub nodes: Vec<AlienRingNodeData>,
}

/// One decoded motion-history sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienRingEntryData {
    /// Pitch increment applied by a node callback.
    pub pitch_step: i16,
    /// Pan increment applied by a node callback.
    pub pan_step: i16,
    /// Radial displacement applied by a node callback.
    pub radial_offset: i16,
    /// Command bits consumed by callback transitions.
    pub command_flags: u16,
}

/// Initial scene-wide motion history shared by every ring model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienRingSceneData {
    /// Shared callback countdown.
    pub timer: u16,
    /// Wrapping generation counter used while allocating model chains.
    pub generation: u16,
    /// Flat ring slot reserved for the next model initialization.
    pub next_ring_slot: usize,
    /// Frames remaining before the captured node advances its resume sequence.
    pub resume_countdown: u16,
    /// Captured node selected for resumption, when authored.
    pub resume_node: Option<AlienModelNodeReference>,
    /// Complete fixed-size motion history.
    pub entries: [AlienRingEntryData; ALIEN_RING_ENTRY_COUNT],
}

/// Callback stage present in an initialized authored slot-2/4 model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSlot2InitialCallbackData {
    /// Advance the species-specific ordinary animation head.
    Update,
    /// Apply AMER's camera-relative autonomous steering.
    AmerSteer,
    /// Complete AMER's camera-relative steering phase.
    AmerFinish,
}

/// Initial semantic callback state for one slot-2/4 model node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienSlot2NodeData {
    /// Species-specific timer, velocity, or follower phase.
    pub motion_parameter: i16,
    /// Desired radial displacement approached by the callback family.
    pub radial_target: u16,
    /// Secondary species-specific timer, target, or captured component.
    pub secondary_motion_parameter: i16,
    /// Deterministic behavior seed used by species-specific transitions.
    pub behavior_seed: u16,
}

/// Initial continuation state for one slot-2/4 animation model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienSlot2ModelData {
    /// Whether the one-time model initializer has already run.
    pub initialized: bool,
    /// Authored callback stage when initialized.
    pub callback: Option<AlienSlot2InitialCallbackData>,
    /// Timer controlling the current callback phase.
    pub phase_timer: i16,
    /// CROOLIS-only signed motion accumulator.
    pub croolis_motion_accumulator: i16,
    /// Sign-extended CROOLIS/SCRUT seed captured during initialization.
    pub species_seed_at_initialization: i32,
    /// Deterministic random value owned by this model.
    pub random_value: u16,
    /// AMER-only wrapped follower-animation phase.
    pub amer_animation_phase: u16,
    /// AMER-only signed return-flight velocity.
    pub amer_velocity: [i16; AXIS_COUNT],
    /// Callback state parallel to the model hierarchy.
    pub nodes: Vec<AlienSlot2NodeData>,
}

/// Initial scene-wide state consumed by slot-2/4 models.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienSlot2SceneData {
    /// Whether a species animation currently owns the camera handoff.
    pub active: bool,
    /// CROOLIS/SCRUT seed remaining after authored model initialization.
    pub species_seed: u16,
}

/// Callback stage stored by an initialized resume method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeCallbackData {
    /// Consume the next queued node or apply idle orientation drift.
    Begin,
    /// Move the resume model toward its queued partner.
    Pair,
    /// Continue texture motion until the shared timeout expires.
    Timeout,
    /// Return to the persistent scene anchor and restart the paired node.
    Final,
}

/// Initial continuation state for one resume behavior model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeMethodData {
    /// Authored callback stage, or `None` before one-time initialization.
    pub callback: Option<AlienResumeCallbackData>,
    /// Packed texture-animation phase.
    pub phase: u16,
    /// Ring node paired with this resume model, when present.
    pub paired_node: Option<AlienModelNodeReference>,
    /// Ring node most recently moved into the timeout stage, when present.
    pub resumed_node: Option<AlienModelNodeReference>,
}

/// Initial scene-wide node queue consumed by the resume behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeSceneData {
    /// Persistent node published by the authored anchor behavior.
    pub anchor_node: Option<AlienModelNodeReference>,
    /// Most recently queued ring node.
    pub current_node: Option<AlienModelNodeReference>,
    /// Queue slot written by ring callbacks.
    pub write_slot: usize,
    /// Queue slot consumed by the resume behavior.
    pub read_slot: usize,
    /// Fixed queue of typed ring-node identities.
    pub queue: [Option<AlienModelNodeReference>; ALIEN_RESUME_QUEUE_CAPACITY],
}

/// One named hierarchical model and its initial behavior method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienModelData {
    /// Eight-character authored model name.
    pub name: String,
    /// Root transform used by the first node.
    pub root: AlienTransformData,
    /// Topologically ordered model nodes.
    pub nodes: Vec<AlienNodeData>,
    /// Model mesh and projection aliases.
    pub mesh: AlienMeshData,
    /// Behavior selected by the model's method table slot.
    pub behavior: AlienBehaviorMethod,
    /// Authored continuation state when this is a wave model.
    pub wave: Option<AlienWaveMethodData>,
    /// Authored continuation state when this is a ring-animation model.
    pub ring: Option<AlienRingModelData>,
    /// Authored continuation state when this is a slot-2/4 animation model.
    pub slot2: Option<AlienSlot2ModelData>,
    /// Authored continuation state when this is a resume behavior model.
    pub resume: Option<AlienResumeMethodData>,
}

/// Indexed atlas shared by all models in one alien overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienTextureAtlas {
    /// Atlas width in texels.
    pub width: usize,
    /// Atlas height in texels.
    pub height: usize,
    /// Row-major palette indices.
    pub pixels: Vec<u8>,
}

/// Complete authored resources currently required by the alien 3D renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienAsset {
    /// Overlay variant that supplied the resources.
    pub kind: AlienXdbKind,
    /// Game revision identified from the overlay's exact native method table.
    pub revision: AlienXdbRevision,
    /// Camera-relative primary mesh rendered before behavior models.
    pub primary_model: AlienPrimaryModelData,
    /// Null-terminated model/context list in authored dispatch order.
    pub models: Vec<AlienModelData>,
    /// Shared indexed texture atlas.
    pub texture: AlienTextureAtlas,
    /// Expanded 8-bit RGB display palette.
    pub palette: [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
    /// Authored palette-index substitutions used to animate texture regions.
    pub palette_remap: [u8; PALETTE_REMAP_ENTRY_COUNT],
    /// Fixed-point trigonometry lookup table.
    pub trigonometry: [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    /// Fixed-point face-raster reciprocal table.
    pub raster_reciprocals: [i32; RASTER_RECIPROCAL_COUNT],
    /// Initial camera and control values.
    pub camera: AlienCameraData,
    /// Initial shared signed delta consumed by behavior methods.
    pub initial_method_delta: i16,
    /// Initial shared input and palette-interaction flags from code data.
    pub initial_scene_flags: u16,
    /// Initial deterministic state shared by animation and ring callbacks.
    pub initial_behavior_random_state: u16,
    /// Initial scene-wide wave selection and sample state.
    pub wave_scene: AlienWaveSceneData,
    /// Initial shared palette-animation continuation state.
    pub palette_animation: AlienPaletteAnimationData,
    /// Initial scene-wide ring timer and motion history.
    pub ring_scene: AlienRingSceneData,
    /// Initial scene-wide slot-2/4 ownership and species seed.
    pub slot2_scene: AlienSlot2SceneData,
    /// Initial typed anchor and queue state consumed by resume behavior.
    pub resume_scene: AlienResumeSceneData,
    /// Distance-to-palette lookup used by the starfield.
    pub star_shade_table: [u8; STAR_SHADE_TABLE_ENTRY_COUNT],
    /// Deterministic seed used to generate the static star distribution.
    pub star_seed: u32,
}

fn read_u16(data: &[u8], position: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(position..position.checked_add(size_of::<u16>())?)?
            .try_into()
            .ok()?,
    ))
}

fn read_i16(data: &[u8], position: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        data.get(position..position.checked_add(size_of::<i16>())?)?
            .try_into()
            .ok()?,
    ))
}

fn read_i32(data: &[u8], position: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(position..position.checked_add(size_of::<i32>())?)?
            .try_into()
            .ok()?,
    ))
}

fn read_u32(data: &[u8], position: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(position..position.checked_add(size_of::<u32>())?)?
            .try_into()
            .ok()?,
    ))
}

fn checked_array<T, const LENGTH: usize>(
    mut element: impl FnMut(usize) -> Option<T>,
) -> Option<[T; LENGTH]> {
    (0..LENGTH)
        .map(&mut element)
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()
}

fn section_start(base: usize, paragraph_delta: u16) -> Option<usize> {
    base.checked_add(usize::from(paragraph_delta).checked_mul(PARAGRAPH_BYTE_COUNT)?)
}

fn transform(data: &[u8], position: usize) -> Option<AlienTransformData> {
    Some(AlienTransformData {
        matrix: checked_array(|row| {
            checked_array(|column| {
                read_i32(
                    data,
                    position + NODE_MATRIX_FIELD + (row * AXIS_COUNT + column) * size_of::<i32>(),
                )
            })
        })?,
        translation: checked_array(|axis| {
            read_i32(
                data,
                position + NODE_TRANSLATION_FIELD + axis * size_of::<i32>(),
            )
        })?,
    })
}

fn model_header(data: &[u8], position: usize) -> Option<String> {
    if data.get(position + MODEL_MAGIC_POSITION..position + MODEL_MAGIC.len())? != MODEL_MAGIC
        || read_u16(data, position + MODEL_HEADER_SIZE_FIELD)? != MODEL_HEADER_SIZE
        || data.get(
            position + MODEL_VERSION_FIELD..position + MODEL_VERSION_FIELD + MODEL_VERSION.len(),
        )? != MODEL_VERSION
    {
        return None;
    }
    let name =
        data.get(position + MODEL_NAME_FIELD..position + MODEL_NAME_FIELD + MODEL_NAME_LENGTH)?;
    let name = std::str::from_utf8(name).ok()?.trim_end_matches('\0');
    (!name.is_empty()).then(|| name.to_owned())
}

fn behavior_method(method_table_offset: u16) -> Option<AlienBehaviorMethod> {
    let offset = usize::from(method_table_offset);
    if offset % METHOD_SLOT_SIZE != usize::MIN {
        return None;
    }
    match offset / METHOD_SLOT_SIZE {
        METHOD_SLOT_NOOP_PRIMARY | METHOD_SLOT_NOOP_SECONDARY | METHOD_SLOT_NOOP_TERTIARY => {
            Some(AlienBehaviorMethod::NoOperation)
        }
        METHOD_SLOT_WAVE => Some(AlienBehaviorMethod::Wave),
        METHOD_SLOT_DISPATCH_PRIMARY | METHOD_SLOT_DISPATCH_SECONDARY => {
            Some(AlienBehaviorMethod::AnimationDispatch)
        }
        METHOD_SLOT_RING => Some(AlienBehaviorMethod::RingAnimation),
        METHOD_SLOT_WRAP_POSITIONS => Some(AlienBehaviorMethod::WrapPositions),
        METHOD_SLOT_PALETTE => Some(AlienBehaviorMethod::PaletteUpdate),
        METHOD_SLOT_SAMPLE_DELTA => Some(AlienBehaviorMethod::ApplySampleDelta),
        METHOD_SLOT_SCALED_SAMPLE_DELTA => Some(AlienBehaviorMethod::ApplyScaledSampleDelta),
        METHOD_SLOT_BOUNDS_WRAP => Some(AlienBehaviorMethod::BoundsThenWrap),
        METHOD_SLOT_ANCHOR => Some(AlienBehaviorMethod::AnchorState),
        METHOD_SLOT_ADJUST_STATE => Some(AlienBehaviorMethod::AdjustState),
        METHOD_SLOT_RESUME => Some(AlienBehaviorMethod::Resume),
        _ => None,
    }
}

fn wave_method_data(
    data: &[u8],
    context: usize,
    behavior: AlienBehaviorMethod,
) -> Option<Option<AlienWaveMethodData>> {
    if behavior != AlienBehaviorMethod::Wave {
        return Some(None);
    }
    Some(Some(AlienWaveMethodData {
        initialized: read_i16(data, context + METHOD_CONTROL_FIELD)? != i16::MIN,
        primary_phase: read_u16(data, context + WAVE_PRIMARY_PHASE_FIELD)?,
        primary_step: read_i16(data, context + WAVE_PRIMARY_STEP_FIELD)?,
        secondary_phase: read_u16(data, context + WAVE_SECONDARY_PHASE_FIELD)?,
        secondary_step: read_i16(data, context + WAVE_SECONDARY_STEP_FIELD)?,
    }))
}

fn ring_slot(cursor: u16) -> Option<usize> {
    let cursor = usize::from(cursor);
    if cursor % RING_ENTRY_SIZE != usize::MIN {
        return None;
    }
    let slot = cursor / RING_ENTRY_SIZE;
    (slot < ALIEN_RING_ENTRY_COUNT).then_some(slot)
}

fn ring_callback(
    callback: u16,
    layout: AlienRingSourceLayout,
) -> Option<AlienRingInitialCallbackData> {
    if callback == layout.initial_course_callback {
        Some(AlienRingInitialCallbackData::InitialCourse)
    } else if callback == layout.follow_course_callback {
        Some(AlienRingInitialCallbackData::FollowCourse)
    } else {
        None
    }
}

fn ring_model_data(
    data: &[u8],
    data_start: usize,
    context: usize,
    root_offset: usize,
    node_count: usize,
    behavior: AlienBehaviorMethod,
    layout: AlienRingSourceLayout,
) -> Option<Option<AlienRingModelData>> {
    if behavior != AlienBehaviorMethod::RingAnimation {
        return Some(None);
    }
    let lifecycle = match read_u16(data, context + METHOD_CONTROL_FIELD)? {
        0 => AlienRingLifecycleData::Uninitialized,
        1 => AlienRingLifecycleData::TimerRunning,
        u16::MAX => AlienRingLifecycleData::TimerSuspended,
        _ => return None,
    };
    let mut nodes = Vec::with_capacity(node_count);
    for node_index in 0..node_count {
        let node_offset = root_offset
            .checked_add(TRANSFORM_RECORD_SIZE)?
            .checked_add(node_index.checked_mul(TRANSFORM_RECORD_SIZE)?)?;
        let position = data_start.checked_add(node_offset)?;
        nodes.push(AlienRingNodeData {
            callback: ring_callback(read_u16(data, position + RING_NODE_CALLBACK_FIELD)?, layout)?,
            course_frames_remaining: read_i16(data, position + RING_NODE_COURSE_FRAMES_FIELD)?,
            feedback_phase: read_u16(data, position + RING_NODE_FEEDBACK_PHASE_FIELD)?,
            ring_slot: ring_slot(read_u16(data, position + RING_NODE_CURSOR_FIELD)?)?,
            behavior_seed: read_u16(data, position + RING_NODE_BEHAVIOR_SEED_FIELD)?,
        });
    }
    Some(Some(AlienRingModelData { lifecycle, nodes }))
}

fn slot2_callback(
    value: u16,
    layout: AlienSlot2SourceLayout,
) -> Option<AlienSlot2InitialCallbackData> {
    if value == layout.update_callback {
        Some(AlienSlot2InitialCallbackData::Update)
    } else if Some(value) == layout.amer_steer_callback {
        Some(AlienSlot2InitialCallbackData::AmerSteer)
    } else if Some(value) == layout.amer_finish_callback {
        Some(AlienSlot2InitialCallbackData::AmerFinish)
    } else {
        None
    }
}

fn slot2_model_data(
    data: &[u8],
    data_start: usize,
    context: usize,
    root_offset: usize,
    node_count: usize,
    behavior: AlienBehaviorMethod,
    kind: AlienXdbKind,
    layout: AlienSlot2SourceLayout,
) -> Option<Option<AlienSlot2ModelData>> {
    if behavior != AlienBehaviorMethod::AnimationDispatch {
        return Some(None);
    }
    let initialized = match read_u16(data, context + METHOD_CONTROL_FIELD)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let callback = if initialized {
        let primary_position = data_start
            .checked_add(root_offset)?
            .checked_add(TRANSFORM_RECORD_SIZE)?;
        Some(slot2_callback(
            read_u16(data, primary_position + SLOT2_NODE_CALLBACK_FIELD)?,
            layout,
        )?)
    } else {
        None
    };
    let mut nodes = Vec::with_capacity(node_count);
    for node_index in 0..node_count {
        let node_offset = root_offset
            .checked_add(TRANSFORM_RECORD_SIZE)?
            .checked_add(node_index.checked_mul(TRANSFORM_RECORD_SIZE)?)?;
        let position = data_start.checked_add(node_offset)?;
        nodes.push(AlienSlot2NodeData {
            motion_parameter: read_i16(data, position + SLOT2_NODE_MOTION_PARAMETER_FIELD)?,
            radial_target: read_u16(data, position + SLOT2_NODE_RADIAL_TARGET_FIELD)?,
            secondary_motion_parameter: read_i16(
                data,
                position + SLOT2_NODE_SECONDARY_MOTION_FIELD,
            )?,
            behavior_seed: read_u16(data, position + SLOT2_NODE_BEHAVIOR_SEED_FIELD)?,
        });
    }
    let (croolis_motion_accumulator, species_seed_at_initialization, random_value) = match kind {
        AlienXdbKind::Amer => (
            i16::default(),
            i32::default(),
            read_u16(data, context + SLOT2_AMER_RANDOM_FIELD)?,
        ),
        AlienXdbKind::Croolis => (
            read_i16(data, context + SLOT2_SECONDARY_CONTEXT_FIELD)?,
            read_i32(data, context + SLOT2_TERTIARY_CONTEXT_FIELD)?,
            read_u16(data, context + SLOT2_COMMON_RANDOM_FIELD)?,
        ),
        AlienXdbKind::Scrut => (
            i16::default(),
            read_i32(data, context + SLOT2_SECONDARY_CONTEXT_FIELD)?,
            read_u16(data, context + SLOT2_COMMON_RANDOM_FIELD)?,
        ),
    };
    Some(Some(AlienSlot2ModelData {
        initialized,
        callback,
        phase_timer: read_i16(data, context + SLOT2_PHASE_TIMER_FIELD)?,
        croolis_motion_accumulator,
        species_seed_at_initialization,
        random_value,
        amer_animation_phase: match kind {
            AlienXdbKind::Amer => read_u16(data, context + SLOT2_AMER_ANIMATION_PHASE_FIELD)?,
            AlienXdbKind::Croolis | AlienXdbKind::Scrut => u16::default(),
        },
        amer_velocity: match kind {
            AlienXdbKind::Amer => checked_array(|axis| {
                read_i16(
                    data,
                    context + SLOT2_SECONDARY_CONTEXT_FIELD + axis * size_of::<i16>(),
                )
            })?,
            AlienXdbKind::Croolis | AlienXdbKind::Scrut => [i16::default(); AXIS_COUNT],
        },
        nodes,
    }))
}

fn slot2_scene_data(data: &[u8], layout: AlienSlot2SourceLayout) -> Option<AlienSlot2SceneData> {
    let active = match read_u16(data, layout.active_position)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    Some(AlienSlot2SceneData {
        active,
        species_seed: match layout.species_seed_position {
            Some(position) => read_u16(data, position)?,
            None => u16::default(),
        },
    })
}

fn resume_callback(value: u16, layout: AlienResumeSourceLayout) -> Option<AlienResumeCallbackData> {
    if value == layout.begin_callback {
        Some(AlienResumeCallbackData::Begin)
    } else if value == layout.pair_callback {
        Some(AlienResumeCallbackData::Pair)
    } else if value == layout.timeout_callback {
        Some(AlienResumeCallbackData::Timeout)
    } else if value == layout.final_callback {
        Some(AlienResumeCallbackData::Final)
    } else {
        None
    }
}

fn optional_model_node_reference(
    data: &[u8],
    data_start: usize,
    context_offsets: &[usize],
    target: u16,
) -> Option<Option<AlienModelNodeReference>> {
    if target == u16::MIN {
        Some(None)
    } else {
        Some(Some(model_node_reference(
            data,
            data_start,
            context_offsets,
            usize::from(target),
        )?))
    }
}

fn resume_method_data(
    data: &[u8],
    data_start: usize,
    context_offsets: &[usize],
    context_offset: usize,
    behavior: AlienBehaviorMethod,
    layout: AlienResumeSourceLayout,
) -> Option<Option<AlienResumeMethodData>> {
    if behavior != AlienBehaviorMethod::Resume {
        return Some(None);
    }
    let context = data_start.checked_add(context_offset)?;
    let callback_value = read_u16(data, context + METHOD_CONTROL_FIELD)?;
    let callback = if callback_value == u16::MIN {
        None
    } else {
        Some(resume_callback(callback_value, layout)?)
    };
    let resumed_node_value = read_u16(data, context + RESUME_RESUMED_NODE_FIELD)?;
    let resumed_node = if resumed_node_value == SHIPPED_UNUSED_RESUMED_NODE_WORD {
        None
    } else {
        optional_model_node_reference(data, data_start, context_offsets, resumed_node_value)?
    };
    Some(Some(AlienResumeMethodData {
        callback,
        phase: read_u16(data, context + RESUME_PHASE_FIELD)?,
        paired_node: optional_model_node_reference(
            data,
            data_start,
            context_offsets,
            read_u16(data, context + RESUME_PAIRED_NODE_FIELD)?,
        )?,
        resumed_node,
    }))
}

fn resume_queue_slot(cursor: u16) -> Option<usize> {
    let cursor = usize::from(cursor);
    if cursor % RESUME_QUEUE_ENTRY_SIZE != usize::MIN {
        return None;
    }
    let slot = cursor / RESUME_QUEUE_ENTRY_SIZE;
    (slot < ALIEN_RESUME_QUEUE_CAPACITY).then_some(slot)
}

fn resume_scene_data(
    data: &[u8],
    data_start: usize,
    context_offsets: &[usize],
    layout: AlienResumeSourceLayout,
) -> Option<AlienResumeSceneData> {
    Some(AlienResumeSceneData {
        anchor_node: optional_model_node_reference(
            data,
            data_start,
            context_offsets,
            read_u16(data, layout.anchor_position)?,
        )?,
        current_node: optional_model_node_reference(
            data,
            data_start,
            context_offsets,
            read_u16(data, layout.current_position)?,
        )?,
        write_slot: resume_queue_slot(read_u16(data, layout.write_cursor_position)?)?,
        read_slot: resume_queue_slot(read_u16(data, layout.read_cursor_position)?)?,
        queue: checked_array(|slot| {
            optional_model_node_reference(
                data,
                data_start,
                context_offsets,
                read_u16(data, layout.queue_position + slot * RESUME_QUEUE_ENTRY_SIZE)?,
            )
        })?,
    })
}

fn ring_scene_data(
    data: &[u8],
    data_start: usize,
    context_offsets: &[usize],
    layout: AlienRingSourceLayout,
) -> Option<AlienRingSceneData> {
    let resume_offset = usize::from(read_u16(data, layout.resume_node_position)?);
    Some(AlienRingSceneData {
        timer: read_u16(data, layout.timer_position)?,
        generation: read_u16(data, layout.generation_position)?,
        next_ring_slot: ring_slot(read_u16(data, layout.cursor_position)?)?,
        resume_countdown: read_u16(data, layout.resume_countdown_position)?,
        resume_node: if resume_offset == usize::MIN {
            None
        } else {
            Some(model_node_reference(
                data,
                data_start,
                context_offsets,
                resume_offset,
            )?)
        },
        entries: checked_array(|index| {
            let position = layout
                .entries_position
                .checked_add(index.checked_mul(RING_ENTRY_SIZE)?)?;
            Some(AlienRingEntryData {
                pitch_step: read_i16(data, position + RING_ENTRY_PITCH_STEP_FIELD)?,
                pan_step: read_i16(data, position + RING_ENTRY_PAN_STEP_FIELD)?,
                radial_offset: read_i16(data, position + RING_ENTRY_RADIAL_OFFSET_FIELD)?,
                command_flags: read_u16(data, position + RING_ENTRY_COMMAND_FLAGS_FIELD)?,
            })
        })?,
    })
}

fn model_node_reference(
    data: &[u8],
    data_start: usize,
    context_offsets: &[usize],
    target: usize,
) -> Option<AlienModelNodeReference> {
    for (model_index, context_offset) in context_offsets.iter().copied().enumerate() {
        let context = data_start.checked_add(context_offset)?;
        let root_offset = usize::from(read_u16(data, context + MODEL_ROOT_FIELD)?);
        let node_count = usize::from(read_u16(data, context + MODEL_NODE_COUNT_FIELD)?);
        for node_index in 0..node_count {
            let node_offset = root_offset
                .checked_add(TRANSFORM_RECORD_SIZE)?
                .checked_add(node_index.checked_mul(TRANSFORM_RECORD_SIZE)?)?;
            if node_offset == target {
                return Some(AlienModelNodeReference {
                    model_index,
                    node_index,
                });
            }
        }
    }
    None
}

fn vertex(data: &[u8], object_start: usize, offset: usize) -> Option<AlienVertexData> {
    let position = object_start.checked_add(offset)?;
    Some(AlienVertexData {
        texture: checked_array(|axis| {
            read_i16(
                data,
                position + VERTEX_TEXTURE_FIELD + axis * size_of::<i16>(),
            )
        })?,
        position: checked_array(|axis| {
            read_i16(
                data,
                position + VERTEX_POSITION_FIELD + axis * size_of::<i16>(),
            )
        })?,
        initial_screen: checked_array(|axis| {
            read_i16(
                data,
                position + VERTEX_SCREEN_FIELD + axis * size_of::<i16>(),
            )
        })?,
        raster_depth: read_i32(data, position + VERTEX_RASTER_DEPTH_FIELD)?,
    })
}

fn faces(
    data: &[u8],
    object_start: usize,
    face_start: usize,
    face_count: usize,
    vertex_indices: &HashMap<usize, usize>,
) -> Option<Vec<AlienFaceData>> {
    let mut faces = Vec::with_capacity(face_count);
    for face_index in 0..face_count {
        let position = object_start
            .checked_add(face_start)?
            .checked_add(face_index.checked_mul(FACE_RECORD_SIZE)?)?;
        faces.push(AlienFaceData {
            vertices: checked_array(|corner| {
                let offset = usize::from(read_u16(
                    data,
                    position + FACE_FIRST_VERTEX_FIELD + corner * size_of::<u16>(),
                )?);
                vertex_indices.get(&offset).copied()
            })?,
        });
    }
    Some(faces)
}

fn primary_model(
    data: &[u8],
    data_start: usize,
    object_start: usize,
) -> Option<AlienPrimaryModelData> {
    let context_offset = usize::from(read_u16(data, data_start + PRIMARY_CONTEXT_POSITION)?);
    let context = data_start.checked_add(context_offset)?;
    let name = model_header(data, context)?;
    let vertex_start = usize::from(read_u16(data, context + PRIMARY_VERTEX_START_FIELD)?);
    let vertex_count = usize::from(read_u16(data, context + PRIMARY_VERTEX_COUNT_FIELD)?);
    let face_start = usize::from(read_u16(data, context + MODEL_FACE_START_FIELD)?);
    let face_count = usize::from(read_u16(data, context + MODEL_FACE_COUNT_FIELD)?);
    if vertex_count == usize::MIN || face_count == usize::MIN {
        return None;
    }

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut vertex_indices = HashMap::with_capacity(vertex_count);
    for index in 0..vertex_count {
        let offset = vertex_start.checked_add(index.checked_mul(VERTEX_RECORD_SIZE)?)?;
        vertex_indices.insert(offset, index);
        vertices.push(vertex(data, object_start, offset)?);
    }
    Some(AlienPrimaryModelData {
        name,
        mesh: AlienMeshData {
            vertices,
            projection_copies: Vec::new(),
            faces: faces(data, object_start, face_start, face_count, &vertex_indices)?,
        },
    })
}

fn model(
    data: &[u8],
    data_start: usize,
    object_start: usize,
    offset: usize,
    kind: AlienXdbKind,
    layout: AlienXdbSourceLayout,
) -> Option<AlienModelData> {
    let context = data_start.checked_add(offset)?;
    let name = model_header(data, context)?;
    let root_offset = usize::from(read_u16(data, context + MODEL_ROOT_FIELD)?);
    let root_position = data_start.checked_add(root_offset)?;
    let node_count = usize::from(read_u16(data, context + MODEL_NODE_COUNT_FIELD)?);
    if node_count == usize::MIN {
        return None;
    }
    let root = transform(data, root_position)?;

    let mut vertices = Vec::new();
    let mut vertex_indices = HashMap::new();
    let mut nodes = Vec::with_capacity(node_count);
    let mut node_indices = HashMap::with_capacity(node_count);
    for node_index in 0..node_count {
        let node_offset = root_offset
            .checked_add(TRANSFORM_RECORD_SIZE)?
            .checked_add(node_index.checked_mul(TRANSFORM_RECORD_SIZE)?)?;
        let position = data_start.checked_add(node_offset)?;
        let parent_offset = usize::from(read_u16(data, position + NODE_PARENT_FIELD)?);
        let parent = match parent_offset {
            SCENE_CAMERA_TRANSFORM_POSITION => AlienNodeParent::SceneCamera,
            _ if parent_offset == root_offset => AlienNodeParent::Root,
            _ => AlienNodeParent::Node(*node_indices.get(&parent_offset)?),
        };
        let first_vertex = vertices.len();
        let vertex_count = usize::from(read_u16(data, position + NODE_VERTEX_COUNT_FIELD)?);
        if vertex_count == usize::MIN {
            return None;
        }
        let vertex_start = usize::from(read_u16(data, position + NODE_VERTEX_START_FIELD)?);
        for vertex_index in 0..vertex_count {
            let vertex_offset =
                vertex_start.checked_add(vertex_index.checked_mul(VERTEX_RECORD_SIZE)?)?;
            if vertex_indices
                .insert(vertex_offset, vertices.len())
                .is_some()
            {
                return None;
            }
            vertices.push(vertex(data, object_start, vertex_offset)?);
        }
        nodes.push(AlienNodeData {
            parent,
            first_vertex,
            vertex_count,
            transform: transform(data, position)?,
            local_position: checked_array(|axis| {
                read_i32(
                    data,
                    position + NODE_LOCAL_POSITION_FIELD + axis * size_of::<i32>(),
                )
            })?,
            angles: checked_array(|axis| {
                read_u16(data, position + NODE_ANGLE_FIELD + axis * size_of::<u16>())
            })?,
            radial_offset: read_i16(data, position + NODE_RADIAL_OFFSET_FIELD)?,
        });
        node_indices.insert(node_offset, node_index);
    }

    let copy_start = usize::from(read_u16(data, context + MODEL_COPY_START_FIELD)?);
    let copy_count = usize::from(read_u16(data, context + MODEL_COPY_COUNT_FIELD)?);
    let mut projection_copies = Vec::with_capacity(copy_count);
    for copy_index in 0..copy_count {
        let copy_offset = copy_start.checked_add(copy_index.checked_mul(VERTEX_RECORD_SIZE)?)?;
        let position = object_start.checked_add(copy_offset)?;
        let source_offset = usize::from(read_u16(data, position + VERTEX_POSITION_FIELD)?);
        let source = *vertex_indices.get(&source_offset)?;
        let destination = vertices.len();
        if vertex_indices.insert(copy_offset, destination).is_some() {
            return None;
        }
        let mut alias = vertex(data, object_start, copy_offset)?;
        alias.position = ZERO_POSITION;
        vertices.push(alias);
        projection_copies.push(AlienProjectionCopyData {
            source,
            destination,
        });
    }

    let face_start = usize::from(read_u16(data, context + MODEL_FACE_START_FIELD)?);
    let face_count = usize::from(read_u16(data, context + MODEL_FACE_COUNT_FIELD)?);
    let behavior = behavior_method(read_u16(data, context + MODEL_METHOD_TABLE_OFFSET_FIELD)?)?;
    let method_slot = usize::from(read_u16(data, context + MODEL_METHOD_TABLE_OFFSET_FIELD)?);
    if read_u16(data, data_start + METHOD_TABLE_POSITION + method_slot)? == INVALID_METHOD_ENTRY {
        return None;
    }

    Some(AlienModelData {
        name,
        root,
        nodes,
        mesh: AlienMeshData {
            vertices,
            projection_copies,
            faces: faces(data, object_start, face_start, face_count, &vertex_indices)?,
        },
        behavior,
        wave: wave_method_data(data, context, behavior)?,
        ring: ring_model_data(
            data,
            data_start,
            context,
            root_offset,
            node_count,
            behavior,
            layout.ring,
        )?,
        slot2: slot2_model_data(
            data,
            data_start,
            context,
            root_offset,
            node_count,
            behavior,
            kind,
            layout.slot2,
        )?,
        resume: None,
    })
}

/// Decode one original alien XDB into flat, typed authored resources.
///
/// Returns `None` if section bounds, model headers, hierarchy topology, method
/// slots, vertex references, palette values, or texture/raster extents are invalid.
pub fn decode_alien_xdb(data: &[u8], kind: AlienXdbKind) -> Option<AlienAsset> {
    let (layout, data_start) = source_layout(data, kind)?;
    let object_start = section_start(
        data_start,
        read_u16(data, data_start + DIRECTORY_OBJECT_DELTA_FIELD)?,
    )?;
    let texture_start = section_start(
        object_start,
        read_u16(data, data_start + DIRECTORY_TEXTURE_DELTA_FIELD)?,
    )?;
    let raster_start = section_start(
        texture_start,
        read_u16(data, data_start + DIRECTORY_RASTER_DELTA_FIELD)?,
    )?;
    let texture_pixel_count = TEXTURE_WIDTH.checked_mul(TEXTURE_HEIGHT)?;
    if texture_start.checked_add(texture_pixel_count)? != raster_start {
        return None;
    }

    let primary_model = primary_model(data, data_start, object_start)?;
    let mut models = Vec::new();
    let mut context_offsets = Vec::new();
    let mut seen_contexts = HashMap::new();
    for index in 0..CONTEXT_LIST_LIMIT {
        let list_position = data_start
            .checked_add(CONTEXT_LIST_POSITION)?
            .checked_add(index.checked_mul(size_of::<u16>())?)?;
        let context_offset = usize::from(read_u16(data, list_position)?);
        if context_offset == usize::MIN {
            break;
        }
        if seen_contexts.insert(context_offset, index).is_some() {
            return None;
        }
        models.push(model(
            data,
            data_start,
            object_start,
            context_offset,
            kind,
            layout,
        )?);
        context_offsets.push(context_offset);
    }
    if models.is_empty() || models.len() == CONTEXT_LIST_LIMIT {
        return None;
    }
    for (model, context_offset) in models.iter_mut().zip(context_offsets.iter().copied()) {
        model.resume = resume_method_data(
            data,
            data_start,
            &context_offsets,
            context_offset,
            model.behavior,
            layout.resume,
        )?;
    }

    let palette_bytes = data.get(
        data_start + DISPLAY_PALETTE_POSITION
            ..data_start + DISPLAY_PALETTE_POSITION + PALETTE_BYTE_COUNT,
    )?;
    let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
    for (entry, source) in palette
        .iter_mut()
        .zip(palette_bytes.chunks_exact(RGB_COMPONENT_COUNT))
    {
        for (component, value) in entry.iter_mut().zip(source) {
            if u16::from(*value) > VGA_DAC_CHANNEL_MAXIMUM {
                return None;
            }
            *component =
                (u16::from(*value) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8;
        }
    }

    let trigonometry = checked_array(|index| {
        let position = data_start + TRIGONOMETRY_POSITION + index * TRIGONOMETRY_RECORD_SIZE;
        Some(AlienTrigonometryPair {
            cosine: read_i16(data, position)?,
            sine: read_i16(data, position + size_of::<i16>())?,
        })
    })?;
    let raster_reciprocals = checked_array(|index| {
        read_i32(
            data,
            raster_start.checked_add(index.checked_mul(size_of::<i32>())?)?,
        )
    })?;
    let camera = AlienCameraData {
        matrix: checked_array(|row| {
            checked_array(|column| {
                read_i32(
                    data,
                    data_start
                        + CAMERA_MATRIX_POSITION
                        + (row * AXIS_COUNT + column) * size_of::<i32>(),
                )
            })
        })?,
        position: checked_array(|axis| {
            read_i32(
                data,
                data_start + CAMERA_POSITION_POSITION + axis * size_of::<i32>(),
            )
        })?,
        transformed_view: checked_array(|axis| {
            read_i32(
                data,
                data_start + CAMERA_RESULT_POSITION + axis * size_of::<i32>(),
            )
        })?,
        angles: checked_array(|axis| {
            read_i16(
                data,
                data_start + CAMERA_ANGLE_POSITION + axis * size_of::<i16>(),
            )
        })?,
        depth_velocity: read_i16(data, data_start + CAMERA_DEPTH_VELOCITY_POSITION)?,
        horizontal_filter: read_i16(data, data_start + CAMERA_HORIZONTAL_FILTER_POSITION)?,
    };
    let star_shade_table = checked_array(|index| {
        data.get(raster_start + kind.star_shade_table_position() + index)
            .copied()
    })?;
    let star_seed = read_u32(data, raster_start + kind.star_seed_position())?;
    let palette_remap =
        checked_array(|index| data.get(layout.palette_remap_position + index).copied())?;
    let wave_scene_position = layout.wave_scene_state_position;
    let selection = match read_u16(data, wave_scene_position)? {
        0 => AlienWaveSelectionData::Disabled,
        1 => AlienWaveSelectionData::Requested,
        2 => AlienWaveSelectionData::Selected,
        _ => return None,
    };
    let selected_offset = usize::from(read_u16(
        data,
        wave_scene_position + WAVE_SELECTED_NODE_FIELD,
    )?);
    let selected_node = if selected_offset == usize::MIN {
        None
    } else {
        Some(model_node_reference(
            data,
            data_start,
            &context_offsets,
            selected_offset,
        )?)
    };

    Some(AlienAsset {
        kind,
        revision: layout.revision,
        primary_model,
        models,
        texture: AlienTextureAtlas {
            width: TEXTURE_WIDTH,
            height: TEXTURE_HEIGHT,
            pixels: data
                .get(texture_start..texture_start + texture_pixel_count)?
                .to_vec(),
        },
        palette,
        palette_remap,
        trigonometry,
        raster_reciprocals,
        camera,
        initial_method_delta: read_i16(data, INITIAL_METHOD_DELTA_POSITION)?,
        initial_scene_flags: match kind {
            AlienXdbKind::Amer => u16::MIN,
            AlienXdbKind::Croolis | AlienXdbKind::Scrut => {
                read_u16(data, OTHER_SCENE_FLAGS_POSITION)?
            }
        },
        initial_behavior_random_state: read_u16(data, data_start + BEHAVIOR_RANDOM_STATE_POSITION)?,
        wave_scene: AlienWaveSceneData {
            selection,
            selected_node,
            current_sample: read_i16(data, wave_scene_position + WAVE_CURRENT_SAMPLE_FIELD)?,
        },
        palette_animation: AlienPaletteAnimationData {
            previous_level: read_u16(data, PALETTE_PREVIOUS_LEVEL_POSITION)?,
            step: *data.get(PALETTE_CYCLE_POSITION + PALETTE_CYCLE_STEP_FIELD)? as i8,
            countdown: *data.get(PALETTE_CYCLE_POSITION + PALETTE_CYCLE_COUNTDOWN_FIELD)?,
            pulse_levels: checked_array(|axis| {
                read_u16(data, data_start + PALETTE_PULSE_POSITIONS[axis])
            })?,
        },
        ring_scene: ring_scene_data(data, data_start, &context_offsets, layout.ring)?,
        slot2_scene: slot2_scene_data(data, layout.slot2)?,
        resume_scene: resume_scene_data(data, data_start, &context_offsets, layout.resume)?,
        star_shade_table,
        star_seed,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
    };

    use super::*;

    const EXPECTED_PRIMARY_VERTEX_COUNT: usize = 83;
    const EXPECTED_PRIMARY_FACE_COUNT: usize = 43;
    const EXPECTED_AMER_MODEL_COUNT: usize = 19;
    const EXPECTED_CROOLIS_MODEL_COUNT: usize = 15;
    const EXPECTED_SCRUT_MODEL_COUNT: usize = 14;
    const ZERO_RASTER_DEPTH: i32 = 0;
    const EXPECTED_INITIAL_METHOD_DELTA: i16 = -4;
    const EXPECTED_PALETTE_PULSE_LEVELS: [u16; AXIS_COUNT] = [10, 13, 11];
    const INITIAL_RING_RADIAL_OFFSET: i16 = 70;

    fn original_xdb(name: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat").join(name),
            Path::new("../../output/_tmp_dat").join(name),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }

    fn big_bug_bang_xdb(name: &str) -> Option<PathBuf> {
        [
            Path::new("output/big-bug-bang/imported-assets/resources").join(name),
            Path::new("../../output/big-bug-bang/imported-assets/resources").join(name),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }

    #[test]
    fn decodes_every_original_alien_scene_into_owned_models() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb", EXPECTED_AMER_MODEL_COUNT),
            (
                AlienXdbKind::Croolis,
                "croolis.xdb",
                EXPECTED_CROOLIS_MODEL_COUNT,
            ),
            (AlienXdbKind::Scrut, "scrut.xdb", EXPECTED_SCRUT_MODEL_COUNT),
        ];

        let mut shared_palette_remap = None;
        for (kind, filename, expected_models) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            assert_eq!(asset.kind, kind);
            assert_eq!(asset.revision, AlienXdbRevision::CommanderBlood);
            assert_eq!(asset.models.len(), expected_models);
            assert_eq!(asset.initial_method_delta, EXPECTED_INITIAL_METHOD_DELTA);
            assert_eq!(
                asset.initial_behavior_random_state,
                match kind {
                    AlienXdbKind::Amer => 0xf2b3,
                    AlienXdbKind::Croolis | AlienXdbKind::Scrut => 0x3cad,
                }
            );
            assert_eq!(asset.palette_animation.previous_level, u16::MIN);
            assert_eq!(asset.palette_animation.step, 1);
            assert_eq!(asset.palette_animation.countdown, 3);
            assert_eq!(asset.initial_scene_flags, u16::MIN);
            assert_eq!(
                asset.palette_animation.pulse_levels,
                EXPECTED_PALETTE_PULSE_LEVELS
            );
            let (primary_phase, secondary_phase, current_sample, selected_node) = match kind {
                AlienXdbKind::Amer => (0x22b4, 0x0b94, 35, None),
                AlienXdbKind::Croolis => (0x1174, 0x05d4, 56, None),
                AlienXdbKind::Scrut => (
                    0x2224,
                    0x0b64,
                    46,
                    Some(AlienModelNodeReference {
                        model_index: 3,
                        node_index: 0,
                    }),
                ),
            };
            assert_eq!(asset.wave_scene.selection, AlienWaveSelectionData::Disabled);
            assert_eq!(asset.wave_scene.selected_node, selected_node);
            assert_eq!(asset.wave_scene.current_sample, current_sample);
            let (
                expected_ring_timer,
                expected_ring_generation,
                expected_next_ring_slot,
                expected_ring_model_count,
                expected_ring_node_count,
                expected_initial_ring_slot,
                expected_initial_course_frames,
                expected_initial_seed,
            ) = match kind {
                AlienXdbKind::Amer => (6, 6, 54, 7, 67, 23, 2, 0xa957),
                AlienXdbKind::Croolis => (2, 3, 90, 4, 34, 11, 2, 0x99f3),
                AlienXdbKind::Scrut => (1, 3, 90, 4, 34, 22, 3, 0xa957),
            };
            assert_eq!(asset.ring_scene.timer, expected_ring_timer);
            assert_eq!(asset.ring_scene.generation, expected_ring_generation);
            assert_eq!(asset.ring_scene.next_ring_slot, expected_next_ring_slot);
            assert_eq!(asset.ring_scene.resume_countdown, u16::MIN);
            assert_eq!(asset.ring_scene.resume_node, None);
            assert_eq!(
                asset.ring_scene.entries[usize::MIN],
                AlienRingEntryData {
                    radial_offset: INITIAL_RING_RADIAL_OFFSET,
                    ..AlienRingEntryData::default()
                }
            );
            let ring_models = asset
                .models
                .iter()
                .filter_map(|model| model.ring.as_ref())
                .collect::<Vec<_>>();
            assert_eq!(ring_models.len(), expected_ring_model_count);
            assert_eq!(
                ring_models
                    .iter()
                    .map(|model| model.nodes.len())
                    .sum::<usize>(),
                expected_ring_node_count
            );
            assert_eq!(
                ring_models[usize::MIN].lifecycle,
                AlienRingLifecycleData::TimerRunning
            );
            assert!(
                ring_models
                    .iter()
                    .skip(1)
                    .all(|model| model.lifecycle == AlienRingLifecycleData::TimerSuspended)
            );
            let ring_nodes = ring_models
                .iter()
                .flat_map(|model| &model.nodes)
                .collect::<Vec<_>>();
            assert_eq!(
                ring_nodes
                    .iter()
                    .filter(|node| { node.callback == AlienRingInitialCallbackData::InitialCourse })
                    .count(),
                1
            );
            assert_eq!(
                ring_nodes
                    .iter()
                    .filter(|node| node.callback == AlienRingInitialCallbackData::FollowCourse)
                    .count(),
                expected_ring_node_count - 1
            );
            let initial_node = ring_nodes
                .iter()
                .find(|node| node.callback == AlienRingInitialCallbackData::InitialCourse)
                .unwrap();
            assert_eq!(initial_node.ring_slot, expected_initial_ring_slot);
            assert_eq!(
                initial_node.course_frames_remaining,
                expected_initial_course_frames
            );
            assert_eq!(initial_node.behavior_seed, expected_initial_seed);
            assert_eq!(
                ring_nodes
                    .iter()
                    .map(|node| node.ring_slot)
                    .collect::<HashSet<_>>()
                    .len(),
                expected_ring_node_count
            );
            assert!(asset.models.iter().all(|model| {
                model.ring.is_some() == (model.behavior == AlienBehaviorMethod::RingAnimation)
            }));
            let (expected_slot2_phases, expected_slot2_seed, expected_slot2_node_count) = match kind
            {
                AlienXdbKind::Amer => (vec![16, 96, 87, 143, 108], 0, 40),
                AlienXdbKind::Croolis => (vec![-43, -43, -43], 500, 24),
                AlienXdbKind::Scrut => (vec![-132, 123, 112], 600, 42),
            };
            assert!(!asset.slot2_scene.active);
            assert_eq!(asset.slot2_scene.species_seed, expected_slot2_seed);
            let slot2_models = asset
                .models
                .iter()
                .filter_map(|model| model.slot2.as_ref())
                .collect::<Vec<_>>();
            assert_eq!(slot2_models.len(), expected_slot2_phases.len());
            assert_eq!(
                slot2_models
                    .iter()
                    .map(|model| model.phase_timer)
                    .collect::<Vec<_>>(),
                expected_slot2_phases
            );
            assert_eq!(
                slot2_models
                    .iter()
                    .map(|model| model.nodes.len())
                    .sum::<usize>(),
                expected_slot2_node_count
            );
            assert!(slot2_models.iter().all(|model| {
                model.initialized
                    && model.callback == Some(AlienSlot2InitialCallbackData::Update)
                    && model.random_value != u16::MIN
            }));
            assert!(asset.models.iter().all(|model| {
                model.slot2.is_some() == (model.behavior == AlienBehaviorMethod::AnimationDispatch)
            }));
            let (expected_anchor_model, expected_resume_read_slot) = match kind {
                AlienXdbKind::Amer => (10, 1),
                AlienXdbKind::Croolis => (9, 5),
                AlienXdbKind::Scrut => (7, 6),
            };
            let resume_models = asset
                .models
                .iter()
                .filter_map(|model| model.resume)
                .collect::<Vec<_>>();
            assert_eq!(
                resume_models,
                vec![AlienResumeMethodData {
                    callback: Some(AlienResumeCallbackData::Begin),
                    phase: u16::MIN,
                    paired_node: None,
                    resumed_node: None,
                }]
            );
            assert!(asset.models.iter().all(|model| {
                model.resume.is_some() == (model.behavior == AlienBehaviorMethod::Resume)
            }));
            assert!(asset.models.iter().all(|model| !matches!(
                model.behavior,
                AlienBehaviorMethod::ApplySampleDelta | AlienBehaviorMethod::ApplyScaledSampleDelta
            )));
            assert_eq!(
                asset.resume_scene.anchor_node,
                Some(AlienModelNodeReference {
                    model_index: expected_anchor_model,
                    node_index: usize::MIN,
                })
            );
            assert_eq!(asset.resume_scene.current_node, None);
            assert_eq!(asset.resume_scene.write_slot, usize::MIN);
            assert_eq!(asset.resume_scene.read_slot, expected_resume_read_slot);
            assert_eq!(
                asset.resume_scene.queue,
                [None; ALIEN_RESUME_QUEUE_CAPACITY]
            );
            let wave_states = asset
                .models
                .iter()
                .filter_map(|model| model.wave)
                .collect::<Vec<_>>();
            assert_eq!(wave_states.len(), 2);
            assert!(wave_states.iter().all(|state| state.initialized));
            assert!(
                wave_states
                    .iter()
                    .all(|state| state.primary_phase == primary_phase)
            );
            assert!(wave_states.iter().all(|state| state.primary_step == 48));
            assert!(
                wave_states
                    .iter()
                    .all(|state| state.secondary_phase == secondary_phase)
            );
            assert!(wave_states.iter().all(|state| state.secondary_step == 16));
            assert_eq!(
                asset.primary_model.mesh.vertices.len(),
                EXPECTED_PRIMARY_VERTEX_COUNT
            );
            assert!(!asset.primary_model.name.is_empty());
            assert_eq!(
                asset.primary_model.mesh.faces.len(),
                EXPECTED_PRIMARY_FACE_COUNT
            );
            assert!(
                asset
                    .primary_model
                    .mesh
                    .vertices
                    .iter()
                    .any(|vertex| vertex.raster_depth != ZERO_RASTER_DEPTH)
            );
            assert_eq!(asset.texture.pixels.len(), TEXTURE_WIDTH * TEXTURE_HEIGHT);
            assert!(
                asset
                    .palette
                    .iter()
                    .flatten()
                    .any(|component| *component != u8::MIN)
            );
            assert!(
                asset
                    .palette_remap
                    .iter()
                    .enumerate()
                    .any(|(index, entry)| usize::from(*entry) != index)
            );
            if let Some(expected) = shared_palette_remap {
                assert_eq!(asset.palette_remap, expected);
            } else {
                shared_palette_remap = Some(asset.palette_remap);
            }
            assert!(
                asset
                    .raster_reciprocals
                    .iter()
                    .any(|value| *value != i32::MIN)
            );
            for model in &asset.models {
                assert!(!model.name.is_empty());
                assert!(!model.nodes.is_empty());
                assert!(!model.mesh.vertices.is_empty());
                assert!(!model.mesh.faces.is_empty());
            }
        }
    }

    #[test]
    fn decodes_authentic_big_bug_bang_alien_scenes() {
        let cases = [
            (AlienXdbKind::Amer, "AMER.XDB", EXPECTED_AMER_MODEL_COUNT),
            (
                AlienXdbKind::Croolis,
                "CROOLIS.XDB",
                EXPECTED_CROOLIS_MODEL_COUNT,
            ),
        ];

        for (kind, filename, expected_models) in cases {
            let Some(path) = big_bug_bang_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            assert_eq!(asset.kind, kind);
            assert_eq!(asset.revision, AlienXdbRevision::BigBugBang);
            assert_eq!(asset.models.len(), expected_models);
            assert_eq!(
                asset.ring_scene.timer,
                match kind {
                    AlienXdbKind::Amer => 7,
                    AlienXdbKind::Croolis => 1,
                    AlienXdbKind::Scrut => unreachable!(),
                }
            );

            let callbacks = asset
                .models
                .iter()
                .filter_map(|model| model.slot2.as_ref())
                .map(|slot2| slot2.callback.unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                callbacks,
                match kind {
                    AlienXdbKind::Amer => vec![
                        AlienSlot2InitialCallbackData::AmerSteer,
                        AlienSlot2InitialCallbackData::AmerFinish,
                        AlienSlot2InitialCallbackData::Update,
                        AlienSlot2InitialCallbackData::AmerSteer,
                        AlienSlot2InitialCallbackData::Update,
                    ],
                    AlienXdbKind::Croolis => {
                        vec![AlienSlot2InitialCallbackData::Update; 3]
                    }
                    AlienXdbKind::Scrut => unreachable!(),
                }
            );
        }
    }

    #[test]
    fn rejects_truncated_and_mismatched_alien_images() {
        assert!(decode_alien_xdb(&[], AlienXdbKind::Amer).is_none());
        let Some(path) = original_xdb("amer.xdb") else {
            return;
        };
        let data = std::fs::read(path).unwrap();
        assert!(decode_alien_xdb(&data, AlienXdbKind::Croolis).is_none());
        assert!(decode_alien_xdb(&data[..data.len() / 2], AlienXdbKind::Amer).is_none());
    }
}
