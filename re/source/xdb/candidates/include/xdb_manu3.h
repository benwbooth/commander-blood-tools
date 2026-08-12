#ifndef XDB_MANU3_H
#define XDB_MANU3_H

#include "xdb_common.h"

#define XDB_MANU3_ACTIVE_SLOTS_OFFSET 0x1032u

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

extern volatile xdb_u16 xdb_manu3_cursor_x; /* DS:0x001A */
extern volatile xdb_u16 XDB_CODE_DATA
        xdb_manu3_data_segment; /* CS:0x136A */
extern volatile xdb_u16
        xdb_manu3_framebuffer_window_offset; /* SS:0x20CE */
extern volatile xdb_u16 xdb_manu3_framebuffer_segment; /* DS:0x0018 */
extern volatile xdb_u16 xdb_manu3_tween_phase; /* DS:0x102C */
extern volatile xdb_u16 xdb_manu3_tween_script_offset; /* DS:0x102E */
extern volatile xdb_u16 xdb_manu3_active_end_offset; /* DS:0x1030 */
extern volatile xdb_u16 xdb_manu3_active_slot_offsets[]; /* DS:0x1032 */
extern volatile xdb_u16 xdb_manu3_active_raster_offset; /* DS:0x0908 */
extern volatile xdb_u16 xdb_manu3_finished_pitch; /* DS:0x223A */
extern volatile xdb_u16 xdb_manu3_finished_yaw; /* DS:0x223C */
extern volatile xdb_u16 xdb_manu3_view_pitch; /* DS:0x23E2 */
extern volatile xdb_u16 xdb_manu3_view_yaw; /* DS:0x23E4 */
extern volatile xdb_u16 xdb_manu3_sequence_table_offset; /* DS:0x2306 */

void XDB_FAR xdb_manu3_anim_select_entry(xdb_u16 selector);
void XDB_FAR xdb_manu3_frame_step(void);
void XDB_NEAR xdb_manu3_anim_select(xdb_u16 selector);
void XDB_NEAR xdb_manu3_tween_step(void);
void XDB_NEAR xdb_manu3_matrix_build(void);
void XDB_NEAR xdb_manu3_entity_project(void);
void XDB_NEAR xdb_manu3_face_builder_next(void);
void XDB_NEAR xdb_manu3_tween_constructor(
        volatile xdb_u16 XDB_NEAR *active_slot_cursor);
void XDB_NEAR xdb_manu3_face_activate(
        const volatile xdb_manu3_face XDB_FAR *face);
void XDB_NEAR xdb_manu3_gradient_setup(
        xdb_u16 vertex_0,
        xdb_u16 vertex_1,
        xdb_u16 vertex_2,
        volatile void XDB_NEAR *raster);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_anim_select_entry \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_frame_step \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_anim_select \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_tween_step \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_tween_constructor \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_face_activate \
        parm [es si] modify exact [ax bx cx dx si di bp]
#endif

#endif
