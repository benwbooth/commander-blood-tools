#ifndef XDB_MANU3_H
#define XDB_MANU3_H

#include "xdb_common.h"

#define XDB_MANU3_ACTIVE_SLOTS_OFFSET 0x1032u
#define XDB_MANU3_BUCKET_HEADS_OFFSET 0x0686u
#define XDB_MANU3_MAX_FACE_WIDTH 0x0190u
#define XDB_MANU3_SCREEN_WIDTH 0x0140u
#define XDB_MANU3_SCREEN_HEIGHT 0x00c8u
#define XDB_MANU3_COLUMN_OFFSET 0x0680u
#define XDB_MANU3_FRAMEBUFFER_COLUMN_OFFSET 0x0682u
#define XDB_MANU3_BUCKET_CURSOR_OFFSET 0x0684u
#define XDB_MANU3_RENDER_CONTINUATION_OFFSET 0x067eu
#define XDB_MANU3_CLIPPED_SORT_HEAD_OFFSET 0x0962u
#define XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET 0x0964u
#define XDB_MANU3_ACTIVE_LIST_MIDDLE_OFFSET 0x09beu
#define XDB_MANU3_ACTIVE_LIST_TAIL_OFFSET 0x0a18u
#define XDB_MANU3_ACTIVE_LIST_ROOT_OFFSET 0x0a28u
#define XDB_MANU3_RASTER_POOL_OFFSET 0x0a72u
#define XDB_MANU3_RASTER_POOL_COUNT 0x00c8u
#define XDB_MANU3_RENDER_FOUR_PLANES_OFFSET 0x0aa4u
#define XDB_MANU3_RENDER_MODE_X_OFFSET 0x0ae0u
#define XDB_MANU3_RENDER_LINEAR_OFFSET 0x0bd6u
#define XDB_MANU3_ADVANCE_COLUMN_OFFSET 0x0873u
#define XDB_MANU3_ADVANCE_SECONDARY_OFFSET 0x0ccau
#define XDB_MANU3_ADVANCE_SWITCH_OFFSET 0x0d19u
#define XDB_MANU3_ADVANCE_REMOVE_OFFSET 0x0d5eu

typedef struct xdb_manu3_tween_spec {
    xdb_u8 count;
    xdb_u8 phase;
    xdb_u16 field_002;
    xdb_u16 target_offset;
    xdb_i16 end_value;
} xdb_manu3_tween_spec;

typedef union xdb_manu3_q16 {
    xdb_u32 raw;
    struct {
        xdb_u16 fraction;
        xdb_i16 whole;
    } parts;
} xdb_manu3_q16;

typedef struct xdb_manu3_tween_record {
    xdb_i16 counter;
    xdb_u16 field_002;
    xdb_u16 target_offset;
    xdb_manu3_q16 accumulator;
    xdb_i32 step;
} xdb_manu3_tween_record;

typedef struct xdb_manu3_face {
    xdb_u16 link;
    xdb_u16 vertex_0;
    xdb_u16 vertex_1;
    xdb_u16 vertex_2;
} xdb_manu3_face;

typedef struct xdb_manu3_cursor_position {
    xdb_i16 x;
    xdb_i16 y;
} xdb_manu3_cursor_position;

typedef struct xdb_manu3_api_request {
    xdb_manu3_cursor_position cursor;
    xdb_u16 animation_selector;
    xdb_u16 framebuffer_window_offset;
} xdb_manu3_api_request;

typedef struct xdb_manu3_point3 {
    xdb_i16 x;
    xdb_i16 y;
    xdb_i16 z;
} xdb_manu3_point3;

typedef struct xdb_manu3_trig_pair {
    xdb_i16 component_0;
    xdb_i16 component_1;
} xdb_manu3_trig_pair;

typedef union xdb_manu3_vertex_field_004 {
    xdb_i16 object_x;
    xdb_u16 projection_source_offset;
} xdb_manu3_vertex_field_004;

typedef union xdb_manu3_screen_position {
    xdb_u32 packed;
    struct {
        xdb_i16 x;
        xdb_i16 y;
    } position;
} xdb_manu3_screen_position;

typedef struct xdb_manu3_vertex {
    xdb_u16 link;
    xdb_u16 field_002;
    xdb_manu3_vertex_field_004 field_004;
    xdb_i16 object_y;
    xdb_i16 object_z;
    xdb_manu3_screen_position screen;
    xdb_i32 depth;
    xdb_u16 clip_flags;
} xdb_manu3_vertex;

typedef union xdb_manu3_texture_coordinate {
    xdb_u32 packed;
    struct {
        xdb_i16 u;
        xdb_i16 v;
    } component;
} xdb_manu3_texture_coordinate;

typedef struct xdb_manu3_raster_record {
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
} xdb_manu3_raster_record;

/* The span builder overlays these boundary nodes on each record at +0 and +0x10. */
typedef struct xdb_manu3_span_boundary {
    xdb_u16 field_000;
    xdb_u16 flags;
    xdb_u16 source_offset;
    xdb_u16 next_boundary_offset;
    xdb_u16 field_008;
    xdb_i16 coordinate;
} xdb_manu3_span_boundary;

typedef char xdb_manu3_raster_record_size_must_be_0x5a[
        sizeof(xdb_manu3_raster_record) == 0x5a ? 1 : -1];
typedef char xdb_manu3_span_boundary_size_must_be_0x0c[
        sizeof(xdb_manu3_span_boundary) == 0x0c ? 1 : -1];

typedef struct xdb_manu3_projection_state {
    xdb_u16 parent_offset;
    xdb_u16 vertex_count;
    xdb_u16 field_004;
    xdb_u16 vertex_offset;
    xdb_u8 field_008[0x0A];
    xdb_i32 matrix[3][3];
    xdb_i32 translation[3];
    xdb_i32 local_position[3];
    xdb_u16 angle_0;
    xdb_u16 angle_1;
    xdb_u16 angle_2;
    xdb_i16 radial_offset;
    xdb_u8 field_056[0x08];
} xdb_manu3_projection_state;

typedef struct xdb_manu3_segment_directory {
    xdb_u16 field_000;
    xdb_u16 work_segment_0;
    xdb_u16 work_segment_1;
    xdb_u16 work_segment_2;
    xdb_u16 field_008;
    xdb_u16 field_00a;
    xdb_u16 work_delta_0;
    xdb_u16 work_delta_1;
    xdb_u16 work_delta_2;
} xdb_manu3_segment_directory;

extern volatile xdb_manu3_cursor_position
        xdb_manu3_cursor; /* DS:0x001A */
extern volatile xdb_u16 XDB_CODE_DATA
        xdb_manu3_data_segment; /* CS:0x136A */
extern volatile xdb_u16 XDB_CODE_DATA
        xdb_manu3_data_segment_delta; /* CS:0x1368 */
extern volatile xdb_manu3_segment_directory
        xdb_manu3_segments; /* DS/FS:0x0000 */
extern const volatile xdb_manu3_trig_pair
        xdb_manu3_trig_table[]; /* DS:0x0026 */
extern volatile xdb_u16 xdb_manu3_angle_scratch_1; /* DS:0x0020 */
extern volatile xdb_u16 xdb_manu3_angle_scratch_0; /* DS:0x0022 */
extern volatile xdb_u16 xdb_manu3_angle_scratch_2; /* DS:0x0024 */
extern volatile xdb_u16
        xdb_manu3_framebuffer_window_offset; /* SS:0x20CE */
extern volatile xdb_u16
        xdb_manu3_linear_framebuffer_segment; /* FS:0x0014 */
extern volatile xdb_u16 xdb_manu3_framebuffer_segment; /* DS:0x0018 */
extern volatile xdb_u16 xdb_manu3_tween_phase; /* DS:0x102C */
extern volatile xdb_u16 xdb_manu3_tween_script_offset; /* DS:0x102E */
extern volatile xdb_u16 xdb_manu3_active_end_offset; /* DS:0x1030 */
extern volatile xdb_u16 xdb_manu3_active_slot_offsets[]; /* DS:0x1032 */
extern volatile xdb_u16 xdb_manu3_active_raster_offset; /* DS:0x0908 */
extern const volatile xdb_i32
        xdb_manu3_reciprocal_table[]; /* raster DS:0x0000 */
extern volatile xdb_u16 xdb_manu3_finished_pitch; /* DS:0x223A */
extern volatile xdb_u16 xdb_manu3_finished_yaw; /* DS:0x223C */
extern volatile xdb_i32 xdb_manu3_screen_center_x; /* DS:0x223E */
extern volatile xdb_i32 xdb_manu3_screen_center_y; /* DS:0x2242 */
extern volatile xdb_u16 xdb_manu3_projection_remaining; /* DS:0x224A */
extern volatile xdb_u16 xdb_manu3_projection_field_224e; /* DS:0x224E */
extern volatile xdb_u16 xdb_manu3_current_state_offset; /* DS:0x2248 */
extern volatile xdb_i32 xdb_manu3_rotation_matrix[3][3]; /* DS:0x2250 */
extern volatile xdb_u16 xdb_manu3_projection_state_count; /* DS:0x22F2 */
extern volatile xdb_u16 xdb_manu3_projection_copy_offset; /* DS/FS:0x22FA */
extern volatile xdb_u16 xdb_manu3_projection_copy_count; /* DS:0x22FE */
extern volatile xdb_u16 xdb_manu3_view_pitch; /* DS:0x23E2 */
extern volatile xdb_u16 xdb_manu3_view_yaw; /* DS:0x23E4 */
extern volatile xdb_u16 xdb_manu3_face_list_offset; /* DS/FS:0x2300 */
extern volatile xdb_u16 xdb_manu3_face_count; /* DS/FS:0x2304 */
extern volatile xdb_u16 xdb_manu3_sequence_table_offset; /* DS:0x2306 */
extern volatile xdb_manu3_projection_state
        xdb_manu3_projection_states[]; /* DS:0x2394 */

void XDB_FAR xdb_manu3_anim_select_entry(xdb_u16 selector);
void XDB_FAR xdb_manu3_api_entry(
        const volatile xdb_manu3_api_request XDB_FAR *request,
        xdb_u16 code_segment);
void XDB_FAR xdb_manu3_init_protocol(xdb_u16 code_segment);
void XDB_FAR xdb_manu3_frame_step(void);
void XDB_NEAR xdb_manu3_anim_select(xdb_u16 selector);
void XDB_NEAR xdb_manu3_tween_step(void);
void XDB_NEAR xdb_manu3_matrix_build(void);
void XDB_NEAR xdb_manu3_entity_project(void);
void XDB_NEAR xdb_manu3_face_builder_next(void);
void XDB_NEAR xdb_manu3_face_bucket_sort(
        xdb_u16 geometry_segment,
        xdb_u16 raster_segment);
void XDB_NEAR xdb_manu3_tween_constructor(
        volatile xdb_u16 XDB_NEAR *active_slot_cursor);
void XDB_NEAR xdb_manu3_face_activate(
        const volatile xdb_manu3_face XDB_FAR *face);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_anim_select_entry \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_init_protocol \
        parm [ax] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_frame_step \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_anim_select \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_tween_step \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_entity_project \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_face_builder_next \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_face_bucket_sort \
        parm [ax] [dx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_tween_constructor \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_face_activate \
        parm [es si] modify exact [ax bx cx dx si di bp]
#endif

#endif
