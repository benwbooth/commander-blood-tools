#ifndef XDB_ALIEN_H
#define XDB_ALIEN_H

#include "xdb_common.h"

#define XDB_ALIEN_CURSOR_BIAS 0x005eu
#define XDB_ALIEN_FIELD_DELTA 0x000fu
#define XDB_ALIEN_RENDER_CONTEXTS_OFFSET 0x2308u
#define XDB_ALIEN_MAX_FACE_WIDTH 0x01f4u
#define XDB_AMER_FACE_BUCKETS_OFFSET 0x094cu
#define XDB_CROOLIS_FACE_BUCKETS_OFFSET 0x094eu
#define XDB_SCRUT_FACE_BUCKETS_OFFSET 0x094eu
#define XDB_ALIEN_BEHIND_SCRATCH_OFFSET 0x07d4u
#define XDB_ALIEN_SCREEN_WIDTH 0x0140u
#define XDB_ALIEN_SCREEN_HEIGHT 0x00c8u
#define XDB_ALIEN_RASTER_POOL_COUNT 0x0258u

#define XDB_AMER_FREE_HEAD_OFFSET 0x0bceu
#define XDB_AMER_COLUMN_OFFSET 0x0946u
#define XDB_AMER_FRAMEBUFFER_COLUMN_OFFSET 0x0948u
#define XDB_AMER_BUCKET_CURSOR_OFFSET 0x094au
#define XDB_AMER_RENDER_CONTINUATION_OFFSET 0x0944u
#define XDB_AMER_CLIPPED_SORT_HEAD_OFFSET 0x0c28u
#define XDB_AMER_ACTIVE_LIST_HEAD_OFFSET 0x0c2au
#define XDB_AMER_ACTIVE_LIST_MIDDLE_OFFSET 0x0c84u
#define XDB_AMER_ACTIVE_LIST_TAIL_OFFSET 0x0cdeu
#define XDB_AMER_ACTIVE_LIST_ROOT_OFFSET 0x0ceeu
#define XDB_AMER_RASTER_POOL_OFFSET 0x0d38u
#define XDB_AMER_RENDER_FOUR_PLANES_OFFSET 0x28a1u
#define XDB_AMER_RENDER_LINEAR_OFFSET 0x29c6u
#define XDB_AMER_ADVANCE_COLUMN_OFFSET 0x2670u
#define XDB_AMER_ADVANCE_SECONDARY_OFFSET 0x2abau
#define XDB_AMER_ADVANCE_SWITCH_OFFSET 0x2b09u
#define XDB_AMER_ADVANCE_REMOVE_OFFSET 0x2b4eu

#define XDB_CROOLIS_FREE_HEAD_OFFSET 0x0bd0u
#define XDB_CROOLIS_COLUMN_OFFSET 0x0948u
#define XDB_CROOLIS_FRAMEBUFFER_COLUMN_OFFSET 0x094au
#define XDB_CROOLIS_BUCKET_CURSOR_OFFSET 0x094cu
#define XDB_CROOLIS_RENDER_CONTINUATION_OFFSET 0x0946u
#define XDB_CROOLIS_CLIPPED_SORT_HEAD_OFFSET 0x0c2au
#define XDB_CROOLIS_ACTIVE_LIST_HEAD_OFFSET 0x0c2cu
#define XDB_CROOLIS_ACTIVE_LIST_MIDDLE_OFFSET 0x0c86u
#define XDB_CROOLIS_ACTIVE_LIST_TAIL_OFFSET 0x0ce0u
#define XDB_CROOLIS_ACTIVE_LIST_ROOT_OFFSET 0x0cf0u
#define XDB_CROOLIS_RASTER_POOL_OFFSET 0x0d3au
#define XDB_CROOLIS_RENDER_FOUR_PLANES_OFFSET 0x2905u
#define XDB_CROOLIS_RENDER_LINEAR_OFFSET 0x2a36u
#define XDB_CROOLIS_ADVANCE_COLUMN_OFFSET 0x26d4u
#define XDB_CROOLIS_ADVANCE_SECONDARY_OFFSET 0x2b2au
#define XDB_CROOLIS_ADVANCE_SWITCH_OFFSET 0x2b79u
#define XDB_CROOLIS_ADVANCE_REMOVE_OFFSET 0x2bbeu

#define XDB_SCRUT_FREE_HEAD_OFFSET 0x0bd0u
#define XDB_SCRUT_COLUMN_OFFSET 0x0948u
#define XDB_SCRUT_FRAMEBUFFER_COLUMN_OFFSET 0x094au
#define XDB_SCRUT_BUCKET_CURSOR_OFFSET 0x094cu
#define XDB_SCRUT_RENDER_CONTINUATION_OFFSET 0x0946u
#define XDB_SCRUT_CLIPPED_SORT_HEAD_OFFSET 0x0c2au
#define XDB_SCRUT_ACTIVE_LIST_HEAD_OFFSET 0x0c2cu
#define XDB_SCRUT_ACTIVE_LIST_MIDDLE_OFFSET 0x0c86u
#define XDB_SCRUT_ACTIVE_LIST_TAIL_OFFSET 0x0ce0u
#define XDB_SCRUT_ACTIVE_LIST_ROOT_OFFSET 0x0cf0u
#define XDB_SCRUT_RASTER_POOL_OFFSET 0x0d3au
#define XDB_SCRUT_RENDER_FOUR_PLANES_OFFSET 0x29c5u
#define XDB_SCRUT_RENDER_LINEAR_OFFSET 0x2af6u
#define XDB_SCRUT_ADVANCE_COLUMN_OFFSET 0x2794u
#define XDB_SCRUT_ADVANCE_SECONDARY_OFFSET 0x2beau
#define XDB_SCRUT_ADVANCE_SWITCH_OFFSET 0x2c39u
#define XDB_SCRUT_ADVANCE_REMOVE_OFFSET 0x2c7eu

typedef struct xdb_alien_biased_state xdb_alien_biased_state;
typedef struct xdb_alien_method_context xdb_alien_method_context;
typedef void XDB_NEAR xdb_alien_state_function(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
typedef xdb_alien_state_function XDB_NEAR *xdb_alien_state_callback;

struct xdb_alien_biased_state {
    xdb_u8 field_000[0x0e];
    xdb_alien_state_callback callback;
    xdb_u8 field_010[0x0a];
    xdb_i32 field_01a;
    xdb_u8 field_01e[0x14];
    xdb_i32 field_032;
    xdb_u8 field_036[0x02];
    xdb_i16 field_038;
    xdb_u8 field_03a[0x02];
    xdb_i16 field_03c;
    xdb_u8 field_03e[0x02];
    xdb_u16 field_040;
    xdb_i32 position_x;
    xdb_i32 position_y;
    xdb_i32 position_z;
    xdb_i16 field_04e;
    xdb_u16 field_050;
    xdb_i16 field_052;
    xdb_i16 field_054;
    xdb_i16 field_056;
    xdb_u16 field_058;
    xdb_u16 ring_offset;
    xdb_u16 field_05c;
};

typedef struct xdb_alien_ring_entry {
    xdb_i16 field_000;
    xdb_i16 field_002;
    xdb_i16 field_004;
    xdb_i16 field_006;
} xdb_alien_ring_entry;

typedef struct xdb_alien_trig_sample {
    xdb_i16 cosine;
    xdb_i16 sine;
} xdb_alien_trig_sample;

typedef union xdb_alien_projection_field_004 {
    xdb_i16 object_x;
    xdb_u16 projection_source_offset;
} xdb_alien_projection_field_004;

typedef union xdb_alien_screen_position {
    xdb_u32 packed;
    struct {
        xdb_i16 x;
        xdb_i16 y;
    } position;
} xdb_alien_screen_position;

typedef struct xdb_alien_projection_vertex {
    xdb_u16 texture_u;
    xdb_u16 texture_v;
    xdb_alien_projection_field_004 field_004;
    xdb_i16 object_y;
    xdb_i16 object_z;
    xdb_alien_screen_position screen;
    xdb_i32 depth;
    xdb_u16 clip_flags;
} xdb_alien_projection_vertex;

typedef union xdb_alien_texture_coordinate {
    xdb_u32 packed;
    struct {
        xdb_i16 u;
        xdb_i16 v;
    } component;
} xdb_alien_texture_coordinate;

typedef struct xdb_alien_face {
    xdb_u16 link;
    xdb_u16 vertex_0;
    xdb_u16 vertex_1;
    xdb_u16 vertex_2;
} xdb_alien_face;

typedef struct xdb_alien_raster_record {
    xdb_u16 next;
    xdb_u16 flags;
    xdb_u16 output_start;
    xdb_u16 output_end;
    xdb_i32 edge_0_position;
    xdb_i32 edge_0_step;
    xdb_u16 previous;
    xdb_u8 field_012[6];
    xdb_i32 edge_1_position;
    xdb_i32 edge_1_step;
    xdb_i32 depth_position;
    xdb_i32 depth_step;
    xdb_i32 depth_gradient;
    xdb_u16 advance_offset;
    xdb_i16 remaining;
    xdb_i16 secondary_remaining;
    xdb_i32 secondary_edge_position;
    xdb_i32 secondary_edge_step;
    xdb_i32 secondary_depth_position;
    xdb_i32 secondary_depth_step;
    xdb_i16 texture_u;
    xdb_i16 texture_v;
    xdb_i16 secondary_texture_u;
    xdb_i16 secondary_texture_v;
    xdb_i16 texture_u_step;
    xdb_i16 texture_v_step;
    xdb_i16 secondary_texture_u_step;
    xdb_i16 secondary_texture_v_step;
    xdb_i16 texture_du;
    xdb_i16 texture_dv;
    xdb_u16 texture_segment;
    xdb_u16 sort_next;
} xdb_alien_raster_record;

typedef struct xdb_alien_span_boundary {
    xdb_u16 field_000;
    xdb_u16 flags;
    xdb_u16 source_offset;
    xdb_u16 next_boundary_offset;
    xdb_u16 field_008;
    xdb_i16 coordinate;
} xdb_alien_span_boundary;

typedef char xdb_alien_raster_record_size_must_be_0x5a[
        sizeof(xdb_alien_raster_record) == 0x5a ? 1 : -1];
typedef char xdb_alien_span_boundary_size_must_be_0x0c[
        sizeof(xdb_alien_span_boundary) == 0x0c ? 1 : -1];

typedef struct xdb_alien_projection_state {
    xdb_u16 parent_offset;
    xdb_u16 vertex_count;
    xdb_u16 field_004;
    xdb_u16 vertex_offset;
    xdb_u8 field_008[0x0a];
    xdb_i32 matrix[3][3];
    xdb_i32 translation[3];
    xdb_i32 local_position[3];
    xdb_u16 angle_0;
    xdb_u16 angle_1;
    xdb_u16 angle_2;
    xdb_i16 radial_offset;
    xdb_u8 field_056[0x08];
} xdb_alien_projection_state;

typedef struct xdb_alien_projection_context {
    xdb_u8 field_000[0x16];
    volatile xdb_alien_projection_state XDB_NEAR *projection_root;
    xdb_u8 field_018[0x02];
    xdb_u16 state_count;
    xdb_u8 field_01c[0x06];
    xdb_u16 copy_offset;
    xdb_u8 field_024[0x02];
    xdb_u16 copy_count;
    xdb_u16 face_offset;
    xdb_u8 field_02a[0x02];
    xdb_u16 face_count;
} xdb_alien_projection_context;

typedef struct xdb_alien_primary_render_context {
    xdb_u8 field_000[0x1c];
    xdb_u16 vertex_offset;
    xdb_u8 field_01e[0x02];
    xdb_u16 vertex_count;
    xdb_u8 field_022[0x06];
    xdb_u16 face_offset;
    xdb_u8 field_02a[0x02];
    xdb_u16 face_count;
} xdb_alien_primary_render_context;

typedef char xdb_alien_projection_vertex_size_must_be_0x14[
        sizeof(xdb_alien_projection_vertex) == 0x14 ? 1 : -1];
typedef char xdb_alien_projection_state_size_must_be_0x5e[
        sizeof(xdb_alien_projection_state) == 0x5e ? 1 : -1];
typedef char xdb_alien_primary_render_context_size_must_be_0x2e[
        sizeof(xdb_alien_primary_render_context) == 0x2e ? 1 : -1];

typedef struct xdb_alien_slot7_root_state {
    xdb_u8 field_000[0x12];
    xdb_i32 field_012;
    xdb_u8 field_016[0x0c];
    xdb_i32 field_022;
    xdb_u8 field_026[0x0c];
    xdb_i32 field_032;
    xdb_i32 field_036;
    xdb_i32 field_03a;
} xdb_alien_slot7_root_state;

typedef struct xdb_alien_slot7_state {
    xdb_alien_slot7_root_state XDB_NEAR *root;
    xdb_u8 field_002[0x34];
    xdb_i32 field_036;
    xdb_i32 field_03a;
    xdb_i32 field_03e;
    xdb_i32 position_x;
    xdb_i32 position_y;
    xdb_u8 field_04a[0x04];
    xdb_i16 mouse_y;
    xdb_u16 mouse_x_0;
    xdb_u16 mouse_x_1;
} xdb_alien_slot7_state;

typedef union xdb_alien_palette_cycle {
    struct {
        xdb_i8 step;
        xdb_i8 countdown;
    } fields;
    xdb_u16 word;
} xdb_alien_palette_cycle;

typedef struct xdb_alien_state {
    xdb_u8 field_000[0x0b0];
    xdb_i16 field_0b0;
} xdb_alien_state;

typedef struct xdb_alien_object_record {
    volatile xdb_i16 position;
    xdb_u8 field_002[0x12];
} xdb_alien_object_record;

typedef struct xdb_alien_wave_object {
    xdb_u8 field_000[0x04];
    xdb_i16 distance;
    xdb_i16 motion;
    xdb_u16 phase;
    xdb_u8 field_00a[0x0a];
} xdb_alien_wave_object;

typedef char xdb_alien_wave_object_size_must_be_0x14[
        sizeof(xdb_alien_wave_object) == 0x14 ? 1 : -1];

typedef void XDB_NEAR xdb_alien_resume_function(
        xdb_alien_method_context XDB_NEAR *context);
typedef xdb_alien_resume_function XDB_NEAR *xdb_alien_resume_callback;

typedef union xdb_alien_method_control {
    xdb_alien_resume_callback resume;
    xdb_i16 state;
} xdb_alien_method_control;

struct xdb_alien_method_context {
    xdb_u8 field_00[0x16];
    volatile xdb_alien_state XDB_NEAR *state;
    xdb_u8 field_018[0x02];
    xdb_u16 state_count;
    xdb_u16 object_offset;
    xdb_u16 field_01e;
    xdb_u16 object_count;
    xdb_u8 field_022[0x14];
    xdb_alien_method_control control;
    union {
        struct {
            xdb_u16 step;
            xdb_u16 value;
        } resume_state;
        struct {
            xdb_u16 cursor;
            xdb_i16 previous;
        } sample_state;
        struct {
            xdb_u16 primary_phase;
            xdb_i16 primary_step;
            xdb_u16 secondary_phase;
            xdb_u16 secondary_step;
        } slot1_wave;
        struct {
            xdb_u16 field_038;
            xdb_u8 field_03a[0x06];
            xdb_u16 random_value;
        } amer_slot2;
        struct {
            xdb_i16 countdown;
            xdb_i16 velocity_x;
            xdb_i16 velocity_y;
            xdb_i16 velocity_z;
            xdb_u16 random_value;
        } amer_slot2_motion;
        struct {
            xdb_u16 duration;
            xdb_u16 field_03a;
            xdb_i32 signed_seed;
            xdb_u8 field_040[0x02];
            xdb_u16 random_value;
        } croolis_slot2;
        struct {
            xdb_u16 duration;
            xdb_i32 signed_seed;
            xdb_u8 field_03e[0x04];
            xdb_u16 random_value;
        } scrut_slot2;
    } continuation;
};

typedef volatile xdb_u8 XDB_NEAR *xdb_alien_cursor;
typedef volatile xdb_alien_biased_state XDB_NEAR *xdb_alien_state_cursor;

extern volatile xdb_i16 XDB_CODE_DATA xdb_alien_method_delta; /* CS:0x0099 */
extern volatile xdb_u16 xdb_alien_object_segment; /* DS:0x0002 */
extern volatile xdb_u16 xdb_alien_palette_segment; /* DS:0x0004 */
#define xdb_alien_texture_segment_base xdb_alien_palette_segment
extern volatile xdb_u16 xdb_alien_raster_segment; /* FS:0x0006; FS=DS invariant */
extern volatile xdb_u16
        xdb_alien_linear_framebuffer_segment; /* FS:0x0024; FS=DS invariant */
extern volatile xdb_u16
        xdb_alien_framebuffer_segment; /* FS:0x0028; FS=DS invariant */
extern volatile xdb_i16 xdb_alien_matrix_angle_pan; /* DS:0x0030 */
extern volatile xdb_i16 xdb_alien_matrix_angle_pitch; /* DS:0x0032 */
extern volatile xdb_i16 xdb_alien_matrix_angle_pan_secondary; /* DS:0x0034 */
extern volatile xdb_u8 xdb_alien_motion_samples[]; /* DS:0x0036 */
extern volatile xdb_alien_trig_sample xdb_alien_angle_table[]; /* DS:0x0036 */
extern volatile xdb_i32 xdb_alien_screen_center_x; /* DS:0x2270 */
extern volatile xdb_i32 xdb_alien_screen_center_y; /* DS:0x2274 */
extern volatile xdb_alien_primary_render_context XDB_NEAR
        *xdb_alien_primary_context_ptr; /* FS:0x2306; FS=DS invariant */
extern volatile xdb_alien_projection_context XDB_NEAR
        *xdb_alien_active_projection_context; /* FS:0x2278; FS=DS invariant */
extern volatile xdb_u16 xdb_alien_current_projection_state_offset; /* DS:0x227A */
extern volatile xdb_u16 xdb_alien_projection_remaining; /* DS:0x227C */
extern volatile xdb_u16 xdb_alien_projection_common_clip; /* DS:0x227E */
extern volatile xdb_u16 xdb_alien_projection_field_2280; /* DS:0x2280 */
extern volatile xdb_i32 xdb_alien_rotation_matrix[3][3]; /* DS:0x2284 */
extern volatile xdb_u16
        xdb_alien_render_context_offsets[]; /* FS:0x2308; FS=DS invariant */
extern volatile xdb_i16 xdb_alien_view_x; /* DS:0x22EC */
extern volatile xdb_i16 xdb_alien_view_y; /* DS:0x22F0 */
extern volatile xdb_i16 xdb_alien_view_z; /* DS:0x22F4 */
extern volatile xdb_i16 xdb_alien_mouse_filter_x; /* DS:0x1058 */
extern volatile xdb_u16 xdb_alien_control_latch; /* DS:0x2282 */
extern volatile xdb_i32 xdb_alien_camera_target_matrix[9]; /* DS:0x2284 */
extern volatile xdb_i32 xdb_alien_camera_matrix[9]; /* DS:0x22BA */
extern volatile xdb_i32 xdb_alien_camera_result[3]; /* DS:0x22DE */
extern volatile xdb_i32 xdb_alien_camera_position[3]; /* DS:0x22EA */
extern volatile xdb_i16 xdb_alien_camera_pitch; /* DS:0x22F6 */
extern volatile xdb_i16 xdb_alien_camera_pan; /* DS:0x22F8 */
extern volatile xdb_i16 xdb_alien_camera_pan_secondary; /* DS:0x22FA */
extern volatile xdb_i16 xdb_alien_camera_depth_step; /* DS:0x22FC */
extern volatile xdb_u16 xdb_alien_exit_requested; /* FS:0x226E; FS=DS invariant */
extern volatile xdb_u16 xdb_alien_random_state; /* FS:0x105C; FS=DS invariant */
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_key_event; /* CS:0x0095 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_code_flags; /* CS:0x02FC */
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_palette_previous_level; /* CS:0x009B */
extern volatile xdb_alien_palette_cycle XDB_CODE_DATA
        xdb_alien_palette_cycle_state; /* CS:0x009F */
extern const volatile xdb_u8 XDB_CODE_DATA
        xdb_amer_palette_remap[256]; /* AMER CS:0x049B */
extern const volatile xdb_u8 XDB_CODE_DATA
        xdb_croolis_palette_remap[256]; /* CROOLIS CS:0x04DC */
extern const volatile xdb_u8 XDB_CODE_DATA
        xdb_scrut_palette_remap[256]; /* SCRUT CS:0x04DC */
extern volatile xdb_u16 xdb_alien_palette_pulse_0; /* DS:0x2536 */
extern volatile xdb_u16 xdb_alien_palette_pulse_1; /* DS:0x2594 */
extern volatile xdb_u16 xdb_alien_palette_pulse_2; /* DS:0x25F2 */
extern xdb_alien_cursor XDB_CODE_DATA
        xdb_amer_slot11_cursor; /* AMER CS:0x1BC2 */
extern xdb_alien_cursor XDB_CODE_DATA
        xdb_croolis_slot11_cursor; /* CROOLIS CS:0x1B2E */
extern xdb_alien_cursor XDB_CODE_DATA
        xdb_scrut_slot11_cursor; /* SCRUT CS:0x1BE3 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_amer_slot3_timer; /* CS:0x0B31 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_croolis_slot3_timer; /* CS:0x0B72 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_scrut_slot3_timer; /* CS:0x0B72 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_amer_slot3_generation; /* CS:0x0D5B */
extern volatile xdb_u16 XDB_CODE_DATA xdb_croolis_slot3_generation; /* CS:0x0DB3 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_scrut_slot3_generation; /* CS:0x0DA1 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_amer_slot3_ring_cursor; /* CS:0x0D5D */
extern volatile xdb_u16 XDB_CODE_DATA xdb_croolis_slot3_ring_cursor; /* CS:0x0DB5 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_scrut_slot3_ring_cursor; /* CS:0x0DA3 */
extern volatile xdb_i16 XDB_CODE_DATA xdb_croolis_slot2_seed; /* CS:0x16A2 */
extern volatile xdb_i16 XDB_CODE_DATA xdb_scrut_slot2_seed; /* CS:0x1690 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_amer_slot2_active; /* CS:0x1648 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_amer_slot1_selection_state; /* CS:0x0B2F */
extern xdb_alien_state_cursor XDB_CODE_DATA
        xdb_amer_slot1_selected_state; /* CS:0x0B33 */
extern volatile xdb_i16 XDB_CODE_DATA xdb_amer_slot1_current_sample; /* CS:0x0B35 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_croolis_slot1_selection_state; /* CS:0x0B70 */
extern xdb_alien_state_cursor XDB_CODE_DATA
        xdb_croolis_slot1_selected_state; /* CS:0x0B74 */
extern volatile xdb_i16 XDB_CODE_DATA xdb_croolis_slot1_current_sample; /* CS:0x0B76 */
extern volatile xdb_u16 XDB_CODE_DATA xdb_scrut_slot1_selection_state; /* CS:0x0B70 */
extern xdb_alien_state_cursor XDB_CODE_DATA
        xdb_scrut_slot1_selected_state; /* CS:0x0B74 */
extern volatile xdb_i16 XDB_CODE_DATA xdb_scrut_slot1_current_sample; /* CS:0x0B76 */
extern volatile xdb_alien_ring_entry XDB_CODE_DATA xdb_amer_slot3_ring[]; /* CS:0x0D63 */
extern volatile xdb_alien_ring_entry XDB_CODE_DATA xdb_croolis_slot3_ring[]; /* CS:0x0DBB */
extern volatile xdb_alien_ring_entry XDB_CODE_DATA xdb_scrut_slot3_ring[]; /* CS:0x0DA9 */

volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_amer_method_slot_11_anchor_state(
        const xdb_alien_method_context XDB_NEAR *context);
volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_croolis_method_slot_11_anchor_state(
        const xdb_alien_method_context XDB_NEAR *context);
volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_scrut_method_slot_11_anchor_state(
        const xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_amer_method_slot_12_apply_delta(
        const xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_croolis_method_slot_12_apply_delta(
        const xdb_alien_method_context XDB_NEAR *context);
volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_scrut_method_slot_12_lower_state(
        const xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_13_resume_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_13_resume_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_13_resume_or_init(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_amer_method_slot_8_apply_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_croolis_method_slot_8_apply_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_scrut_method_slot_8_apply_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_amer_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_croolis_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_scrut_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_10_bounds_then_wrap(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_10_bounds_then_wrap(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_10_bounds_then_wrap(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_2_dispatch_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_2_4_dispatch_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_2_4_dispatch_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_3_update_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_3_update_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_3_update_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_1_wave_update_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_1_wave_update_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_1_wave_update_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_mouse_camera_step(void);
void XDB_NEAR xdb_croolis_mouse_camera_step(void);
void XDB_NEAR xdb_scrut_mouse_camera_step(void);
void XDB_NEAR xdb_amer_camera_matrix_update(void);
void XDB_NEAR xdb_croolis_camera_matrix_update(void);
void XDB_NEAR xdb_scrut_camera_matrix_update(void);
void XDB_NEAR xdb_amer_transform_and_project(void);
void XDB_NEAR xdb_croolis_transform_and_project(void);
void XDB_NEAR xdb_scrut_transform_and_project(void);
void XDB_NEAR xdb_amer_project_primary_mesh_then_render(void);
void XDB_NEAR xdb_croolis_project_primary_mesh_then_render(void);
void XDB_NEAR xdb_scrut_project_primary_mesh_then_render(void);
void XDB_NEAR xdb_amer_bucket_faces_then_render(void);
void XDB_NEAR xdb_croolis_bucket_faces_then_render(void);
void XDB_NEAR xdb_scrut_bucket_faces_then_render(void);
void XDB_NEAR xdb_amer_render_face_buckets(void);
void XDB_NEAR xdb_croolis_render_face_buckets(void);
void XDB_NEAR xdb_scrut_render_face_buckets(void);
void XDB_NEAR xdb_amer_face_activate(
        const volatile xdb_alien_face XDB_FAR *face,
        xdb_u16 raster_segment);
void XDB_NEAR xdb_croolis_face_activate(
        const volatile xdb_alien_face XDB_FAR *face,
        xdb_u16 raster_segment);
void XDB_NEAR xdb_scrut_face_activate(
        const volatile xdb_alien_face XDB_FAR *face,
        xdb_u16 raster_segment);
void XDB_NEAR xdb_amer_method_slot_7_palette_update(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_7_palette_update(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_7_palette_update(
        xdb_alien_method_context XDB_NEAR *context);

void XDB_NEAR xdb_amer_resume_1c34(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_resume_1b85(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_resume_1c45(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_slot3_initial_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_slot3_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_slot3_initial_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_slot3_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_slot3_initial_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_slot3_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_slot2_return_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_slot2_steer_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_slot2_finish_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_alien_resume_function parm [di]
#pragma aux xdb_alien_state_function parm [si] [di] modify exact [ax bx cx dx]
#pragma aux xdb_amer_slot2_return_update \
        parm [si] [di] modify exact [ax bx cx dx]
#pragma aux xdb_amer_slot2_steer_update \
        parm [si] [di] modify exact [ax bx cx dx]
#pragma aux xdb_amer_method_slot_11_anchor_state parm [di] value [si] modify exact [si]
#pragma aux xdb_croolis_method_slot_11_anchor_state parm [di] value [si] modify exact [si]
#pragma aux xdb_scrut_method_slot_11_anchor_state parm [di] value [si] modify exact [si]
#pragma aux xdb_amer_method_slot_12_apply_delta parm [di] value [ax] modify exact [ax si]
#pragma aux xdb_croolis_method_slot_12_apply_delta parm [di] value [ax] modify exact [ax si]
#pragma aux xdb_scrut_method_slot_12_lower_state parm [di] value [si] modify exact [si]
#pragma aux xdb_amer_method_slot_13_resume_or_init parm [di]
#pragma aux xdb_croolis_method_slot_13_resume_or_init parm [di]
#pragma aux xdb_scrut_method_slot_13_resume_or_init parm [di]
#pragma aux xdb_amer_method_slot_8_apply_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_croolis_method_slot_8_apply_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_scrut_method_slot_8_apply_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_amer_method_slot_9_apply_scaled_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_croolis_method_slot_9_apply_scaled_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_scrut_method_slot_9_apply_scaled_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_amer_method_slot_6_wrap_positions \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_method_slot_6_wrap_positions \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_method_slot_6_wrap_positions \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_amer_method_slot_10_bounds_then_wrap \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_method_slot_10_bounds_then_wrap \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_method_slot_10_bounds_then_wrap \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_amer_method_slot_2_dispatch_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_method_slot_2_4_dispatch_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_method_slot_2_4_dispatch_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_amer_method_slot_3_update_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_method_slot_3_update_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_method_slot_3_update_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_amer_method_slot_1_wave_update_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_method_slot_1_wave_update_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_method_slot_1_wave_update_or_init \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_amer_mouse_camera_step modify exact [ax bx cx dx]
#pragma aux xdb_croolis_mouse_camera_step modify exact [ax bx cx dx]
#pragma aux xdb_scrut_mouse_camera_step modify exact [ax bx cx dx]
#pragma aux xdb_amer_camera_matrix_update \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_camera_matrix_update \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_camera_matrix_update \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_amer_transform_and_project \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_transform_and_project \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_scrut_transform_and_project \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_amer_project_primary_mesh_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_project_primary_mesh_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_scrut_project_primary_mesh_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_amer_bucket_faces_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_bucket_faces_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_scrut_bucket_faces_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_amer_render_face_buckets \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_render_face_buckets \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_scrut_render_face_buckets \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_amer_face_activate \
        parm [es si] [dx] modify exact [ax bx cx dx si di]
#pragma aux xdb_croolis_face_activate \
        parm [es si] [dx] modify exact [ax bx cx dx si di]
#pragma aux xdb_scrut_face_activate \
        parm [es si] [dx] modify exact [ax bx cx dx si di]
#pragma aux xdb_amer_method_slot_7_palette_update \
        parm [di] modify exact [ax bx cx dx si es]
#pragma aux xdb_croolis_method_slot_7_palette_update \
        parm [di] modify exact [ax bx cx dx si es]
#pragma aux xdb_scrut_method_slot_7_palette_update \
        parm [di] modify exact [ax bx cx dx si es]
#endif

#endif
