#ifndef BLOODPRG_SHIP3D_H
#define BLOODPRG_SHIP3D_H

#include "bloodprg_common.h"
#include "bloodprg_vm.h"

#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8 0x0008u
#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10 0x0010u
#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40 0x0040u
#define SHIP_3D_OBJECT_KIND_POSITION_KIND100 0x0100u
#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200 0x0200u

#define SHIP_3D_FIELD_SELECTOR_POSITION 0x000bu
#define SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH 0x0009u
#define SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH 0x000au
#define SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD 0x000cu
#define SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD 0x000eu
#define SHIP_3D_FIELD_SELECTOR_PARENT_LINK 0x0011u
#define SHIP_3D_SOURCE_BITSET_SELECTOR 0x0005u
#define SHIP_3D_SOURCE_BITSET_KIND 0x0002u
#define SHIP_3D_HUD_PALETTE_FIRST 128u
#define SHIP_3D_HUD_PALETTE_COLORS 64u
#define SHIP_3D_HUD_PALETTE_BYTES (SHIP_3D_HUD_PALETTE_COLORS * 3u)

typedef struct ship_3d_projection_context {
    cb_i32 matrix[9];
    cb_u16 projected_x;
    cb_u16 projected_y;
    cb_u16 projected_depth;
} ship_3d_projection_context;

typedef struct ship_3d_projection_terms {
    cb_i32 b_cos;
    cb_i32 b_sin;
    cb_i32 c_cos;
    cb_i32 c_sin;
    cb_i32 a_cos;
    cb_i32 a_sin;
} ship_3d_projection_terms;

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

typedef struct ship_3d_position_field {
    cb_u16 x;
    cb_u16 y;
} ship_3d_position_field;

extern volatile cb_u16 CB_GAME_DATA ship_3d_depth_offset; /* DS/GS:0x2527 */
extern volatile cb_u8 CB_GAME_DATA
        ship_3d_plane_blit_crop_enabled; /* DS/GS:0x252E */
extern volatile cb_u8 ship_3d_depth_opening; /* DS:0x252F */
extern volatile cb_u8 ship_3d_depth_closing; /* DS:0x2530 */
extern volatile cb_u8 ship_3d_depth_step;    /* DS:0x2531 */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_angle_b; /* GS:0x2F6D */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_angle_c; /* GS:0x2F6F */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_angle_a; /* GS:0x2F71 */
/* Original BP addressing selects SS:0x2A1B; medium-model C requires SS == DS. */
extern volatile ship_3d_matrix_slot ship_3d_matrix_slots[];
extern volatile ship_3d_projection_terms CB_GAME_DATA ship_3d_projection_inputs; /* GS:0x2F7D */
extern volatile ship_3d_projection_context CB_GAME_DATA ship_3d_projection; /* GS:0x2F95 */
extern volatile ship_3d_point_record CB_GAME_DATA ship_3d_point_cloud[]; /* GS:0x2FC1 */
/* Original BP indexing selects SS:0x4F45; GAME_DATA must bind to SS == GS. */
extern const ship_3d_angle_table_entry CB_GAME_DATA ship_3d_angle_table[];
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_left;    /* GS:0x5235 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_right;   /* GS:0x5237 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_top;     /* GS:0x5239 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_bottom;  /* GS:0x523B */
extern volatile cb_u8 CB_GAME_DATA
        ship_3d_hud_palette_snapshot[SHIP_3D_HUD_PALETTE_BYTES]; /* GS:0x5CD8 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_camera_x; /* GS:0x2F65 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_camera_y; /* GS:0x2F67 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_camera_z; /* GS:0x2F69 */

#if defined(__WATCOMC__)
#pragma aux binary_u32_sqrt parm [dx ax] value [ax] modify exact [ax]
#pragma aux ship_3d_position_distance parm [si] [di] [dx] value [ax] modify exact [ax]
#pragma aux ship_3d_position_field_resolve parm [si] [dx] value [ax] modify exact [ax]
#pragma aux ship_3d_object_table_bit_test_full parm [ax] [si] value [ax] modify exact [ax]
/* Open Watcom C16 reserves BP, so the natural-C boundary uses BX for the
 * output cursor; an integration adapter must translate the binary's BP ABI. */
#pragma aux ship_3d_nav_source_list_build_full parm [es di] [bx] value [bx] modify exact [bx]
#pragma aux matrix_table_clear_2a1b modify exact []
#pragma aux ship_3d_projection_matrix_build modify exact [ax es]
#pragma aux ship_3d_point_cloud_randomize modify exact [ax cx es]
#pragma aux ship_3d_depth_scroll_step modify exact [ax]
#pragma aux ship_3d_hud_palette_snapshot_and_camera_reset \
        modify exact [bx dx]
#pragma aux ship_3d_point_cloud_project modify exact []
#pragma aux ship_3d_object_sprite_project modify exact []
#pragma aux bridge_panorama_frame_load parm [ax] modify exact []
#pragma aux page_flip value [ax] modify exact [ax bx]
#endif

cb_u16 CB_FAR binary_u32_sqrt(cb_u32 value); /* 0x002E33 */
cb_u16 CB_NEAR ship_3d_position_distance(
        const volatile bloodprg_vm_object_header CB_NEAR *first_record,
        const volatile bloodprg_vm_object_header CB_NEAR *second_record,
        cb_u16 inherited_kind100_compare_word); /* 0x0060DD */
volatile ship_3d_position_field CB_NEAR *CB_NEAR
ship_3d_position_field_resolve(
        volatile bloodprg_vm_object_header CB_NEAR *record,
        cb_u16 kind100_compare_word);        /* 0x0061A6 */
int CB_NEAR ship_3d_object_table_bit_test_full(cb_u16 object_offset,
        const volatile cb_u8 CB_NEAR *bitset_base); /* 0x006210 */
cb_u16 CB_NEAR *CB_FAR ship_3d_nav_source_list_build_full(
        const volatile bloodprg_vm_object_header CB_FAR *target,
        cb_u16 CB_NEAR *output);              /* 0x00624B */
void CB_FAR matrix_table_clear_2a1b(void);     /* 0x00963F */
void CB_FAR ship_3d_projection_matrix_build(void); /* 0x0098B9 */
void CB_FAR ship_3d_point_cloud_project(void); /* 0x009A10 */
void CB_FAR ship_3d_object_sprite_project(void); /* 0x009B98 */
void CB_FAR ship_3d_point_cloud_randomize(void); /* 0x009B67 */
void CB_NEAR bridge_panorama_frame_load(cb_u16 frame); /* 0x00981B */
cb_u16 CB_FAR page_flip(void); /* 0x00954A */
void CB_NEAR ship_3d_depth_scroll_step(void); /* 0x00B75C */
void CB_FAR draw_hud_element_2bc7(void); /* 0x006FF3 */
void CB_FAR ship_3d_hud_palette_snapshot_and_camera_reset(void); /* 0x008C96 */
/* Original context is SS:BP and the framebuffer is normalized ES:0. */
void CB_NEAR ship_3d_plot_point(
        const volatile ship_3d_projection_context CB_GAME_DATA *projection,
        volatile cb_u8 CB_FAR *framebuffer);   /* 0x009B04 */

#endif
