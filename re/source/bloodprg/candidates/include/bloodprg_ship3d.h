#ifndef BLOODPRG_SHIP3D_H
#define BLOODPRG_SHIP3D_H

#include "bloodprg_common.h"

typedef struct ship_3d_projection_context {
    cb_i32 matrix[9];
    cb_u16 projected_x;
    cb_u16 projected_y;
    cb_u16 projected_depth;
} ship_3d_projection_context;

typedef struct ship_3d_angle_table_entry {
    cb_i16 cosine;
    cb_i16 sine;
} ship_3d_angle_table_entry;

typedef struct ship_3d_point_record {
    cb_u16 x;
    cb_u16 y;
    cb_u16 z;
    cb_u16 scratch;
} ship_3d_point_record;

typedef struct ship_3d_matrix_slot {
    cb_u16 first_word;
    cb_u8 tail[22];
} ship_3d_matrix_slot;

extern volatile cb_u16 ship_3d_depth_offset; /* GS:0x2527 */
extern volatile cb_u8 ship_3d_depth_opening; /* GS:0x252F */
extern volatile cb_u8 ship_3d_depth_closing; /* GS:0x2530 */
extern volatile cb_u8 ship_3d_depth_step;    /* GS:0x2531 */
extern volatile cb_u16 ship_3d_projection_angle_b; /* GS:0x2F6D */
extern volatile cb_u16 ship_3d_projection_angle_c; /* GS:0x2F6F */
extern volatile cb_u16 ship_3d_projection_angle_a; /* GS:0x2F71 */
extern volatile ship_3d_matrix_slot ship_3d_matrix_slots[]; /* GS:0x2A1B */
extern volatile ship_3d_projection_context ship_3d_projection; /* GS:0x2F95 */
extern volatile ship_3d_point_record ship_3d_point_cloud[]; /* GS:0x2FC1 */
extern const ship_3d_angle_table_entry ship_3d_angle_table[]; /* GS:0x4F45 */
extern volatile cb_u32 ship_3d_render_state_block[]; /* GS:0x5251 */
extern volatile cb_i16 ship_3d_clip_left;    /* GS:0x5235 */
extern volatile cb_i16 ship_3d_clip_right;   /* GS:0x5237 */
extern volatile cb_i16 ship_3d_clip_top;     /* GS:0x5239 */
extern volatile cb_i16 ship_3d_clip_bottom;  /* GS:0x523B */

#endif
