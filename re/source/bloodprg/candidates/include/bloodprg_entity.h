#ifndef BLOODPRG_ENTITY_H
#define BLOODPRG_ENTITY_H

#include "bloodprg_common.h"

#define BLOODPRG_ENTITY_STATE0_FLAG 0x0001u
#define BLOODPRG_ENTITY_DIRTY_FLAG 0x0002u
#define BLOODPRG_ENTITY_EXTENT_CHANGED_FLAG 0x0010u
#define BLOODPRG_ENTITY_ACTIVE_FLAG 0x0080u
#define BLOODPRG_ENTITY_ACTIVE_OR_STATE0_MASK 0x0081u
#define BLOODPRG_ENTITY_RESOURCE_FLAG 0x0004u
#define BLOODPRG_ENTITY_ACTIVATE_FLAGS 0x0083u

typedef union bloodprg_entity_flags {
    cb_u16 word;
    struct {
        cb_u8 low;
        cb_u8 high;
    } bytes;
} bloodprg_entity_flags;

typedef struct bloodprg_dirty_rect {
    cb_u16 left;
    cb_u16 right;
    cb_u16 top;
    cb_u16 bottom;
} bloodprg_dirty_rect;

typedef struct bloodprg_sprite_frame {
    cb_u16 stride;
    cb_u16 height;
    cb_i16 x_offset;
    cb_i16 y_offset;
    cb_u8 pixels[1];
} bloodprg_sprite_frame;

typedef struct bloodprg_entity_resource {
    cb_u16 flags;
    cb_i16 frame_count;
    cb_u32 packed_frame_offsets[1];
} bloodprg_entity_resource;

typedef struct bloodprg_entity_record {
    cb_u16 flags;
    cb_u16 field_02;
    const volatile bloodprg_sprite_frame CB_FAR *frame;
    cb_u16 draw_x;
    cb_u16 draw_y;
    cb_u16 extent_width;
    cb_u16 extent_height;
    cb_u16 committed_draw_x;
    cb_u16 committed_draw_y;
    cb_u16 committed_extent_width;
    cb_u16 committed_extent_height;
    bloodprg_dirty_rect dirty_rect;
} bloodprg_entity_record;

typedef struct bloodprg_sprite_source_extent {
    cb_u16 width;
    cb_u16 height;
} bloodprg_sprite_source_extent;

typedef void CB_NEAR bloodprg_sprite_blitter(
        volatile bloodprg_entity_record *record);

extern volatile bloodprg_entity_record CB_GAME_DATA
        bloodprg_entity_table[]; /* GS:0x6212 */
extern volatile bloodprg_dirty_rect bloodprg_clip_bounds; /* GS:0x5235 */
extern volatile cb_u16 bloodprg_clip_snapshot_flags; /* GS:0x5249 */
extern volatile bloodprg_dirty_rect bloodprg_dirty_rect_list[]; /* GS:0x6612 */
extern bloodprg_sprite_blitter *bloodprg_sprite_blitter_table[8]; /* CS:0x1592 */
extern bloodprg_sprite_blitter *bloodprg_selected_sprite_blitter; /* CS:0x15A2 */
extern volatile cb_u8 bloodprg_sprite_flip_x; /* CS:0x14DF */
extern volatile cb_u8 bloodprg_sprite_flip_y; /* CS:0x14E0 */
extern volatile cb_u8 CB_FAR *bloodprg_display_buffer; /* GS:0x5221 */
extern volatile cb_u8 CB_FAR *bloodprg_secondary_buffer; /* GS:0x5229 */
extern volatile cb_u8 bloodprg_dirty_copy_flags; /* GS:0x5231 */
extern volatile cb_u8 bloodprg_sprite_remap_5f11[256]; /* GS:0x5F11 */
extern volatile cb_u8 bloodprg_sprite_remap_6011[256]; /* GS:0x6011 */
extern volatile cb_u8 CB_NEAR *bloodprg_selected_sprite_remap; /* GS:0x524B */
extern volatile cb_u16 bloodprg_rle_stride; /* CS:0x1726 */
extern volatile cb_u16 bloodprg_rle_left_clip; /* CS:0x1728 */
extern volatile cb_u16 bloodprg_rle_right_clip; /* CS:0x172A */

void CB_FAR entity_flag_state_transition(cb_u16 object_id); /* 0x0299:0x1241 */
void CB_FAR sprite_slot_position_update(cb_u16 object_id,
        cb_u16 draw_x,
        cb_u16 draw_y); /* 0x0299:0x127D */
/* 0x0299:0x133D; source_extent normalizes the inherited SS:BP+4 context. */
void CB_FAR sprite_slot_extent_update(cb_u16 object_id,
        cb_u16 width,
        cb_u16 height,
        const volatile bloodprg_sprite_source_extent CB_FAR *source_extent);
void CB_FAR sprite_slot_range_mark_dirty(cb_u16 first_object_id,
        cb_u16 last_object_id); /* 0x0299:0x12B0 */
void CB_FAR sprite_slot_commit_dirty_range(cb_u16 first_object_id,
        cb_u16 last_object_id); /* 0x0299:0x1467 */
void CB_FAR sprite_slot_dirty_range_render(cb_u16 first_object_id,
        cb_u16 last_object_id); /* 0x0299:0x14E1 */
void CB_NEAR sprite_blit_raw_transparent(
        volatile bloodprg_entity_record *record); /* 0x0299:0x15A6 */
void CB_NEAR sprite_blit_rle_transparent(
        volatile bloodprg_entity_record *record); /* 0x0299:0x172C */
void CB_NEAR sprite_blit_raw_opaque(
        volatile bloodprg_entity_record *record); /* 0x0299:0x1C18 */
void CB_NEAR sprite_blit_rle_opaque(
        volatile bloodprg_entity_record *record); /* 0x0299:0x1D46 */
void CB_NEAR sprite_blit_scaled_transparent(
        volatile bloodprg_entity_record *record); /* 0x0299:0x1FD2 */
void CB_NEAR sprite_blitter_noop_5(
        volatile bloodprg_entity_record *record); /* 0x0299:0x210A */
void CB_NEAR sprite_blitter_noop_6(
        volatile bloodprg_entity_record *record); /* 0x0299:0x210B */
void CB_NEAR sprite_blitter_noop_7(
        volatile bloodprg_entity_record *record); /* 0x0299:0x210C */
void CB_FAR dirty_rects_copy_secondary_to_primary(
        const volatile bloodprg_dirty_rect CB_FAR *rectangles);
        /* 0x0299:0x210D */

void CB_FAR entity_record_setter(cb_u16 entity_id,
        const volatile void CB_FAR *resource,
        cb_u16 draw_x,
        cb_u16 draw_y,
        cb_u16 frame_index); /* 0x0299:0x11BE */
void CB_FAR entity_object_populate(cb_u16 entity_id,
        cb_u16 resource_handle,
        cb_u16 draw_x,
        cb_u16 draw_y,
        cb_u16 frame_index); /* 0x0299:0x1140 */

#if defined(__WATCOMC__)
#pragma aux bloodprg_sprite_blitter parm [di] modify exact []
#pragma aux entity_object_populate \
        parm caller [ax] [dx] [bx] [cx] modify exact []
#pragma aux entity_record_setter \
        parm caller [ax] [es di] [bx] [cx] modify exact []
#pragma aux entity_flag_state_transition parm [ax]
#pragma aux sprite_slot_position_update parm [ax] [bx] [cx]
#pragma aux sprite_slot_extent_update parm [ax] [cx] [dx] [es si]
#pragma aux sprite_slot_range_mark_dirty parm [ax] [bx]
#pragma aux sprite_slot_commit_dirty_range parm [ax] [bx]
#pragma aux sprite_slot_dirty_range_render parm [ax] [bx]
#pragma aux sprite_blit_raw_transparent parm [di] modify exact []
#pragma aux sprite_blit_rle_transparent parm [di] modify exact []
#pragma aux sprite_blit_raw_opaque parm [di] modify exact []
#pragma aux sprite_blit_rle_opaque parm [di] modify exact []
#pragma aux sprite_blit_scaled_transparent parm [di] modify exact []
#pragma aux sprite_blitter_noop_5 parm [di] modify exact []
#pragma aux sprite_blitter_noop_6 parm [di] modify exact []
#pragma aux sprite_blitter_noop_7 parm [di] modify exact []
#pragma aux dirty_rects_copy_secondary_to_primary parm [es di] modify exact []
#endif

#endif
