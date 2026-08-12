#ifndef BLOODPRG_ENTITY_H
#define BLOODPRG_ENTITY_H

#include "bloodprg_common.h"

#define BLOODPRG_ENTITY_STATE0_FLAG 0x0001u
#define BLOODPRG_ENTITY_DIRTY_FLAG 0x0002u
#define BLOODPRG_ENTITY_EXTENT_CHANGED_FLAG 0x0010u
#define BLOODPRG_ENTITY_ACTIVE_FLAG 0x0080u
#define BLOODPRG_ENTITY_ACTIVE_OR_STATE0_MASK 0x0081u

typedef union bloodprg_entity_flags {
    cb_u16 word;
    struct {
        cb_u8 low;
        cb_u8 high;
    } bytes;
} bloodprg_entity_flags;

typedef struct bloodprg_entity_record {
    cb_u16 flags;
    cb_u16 field_02;
    cb_u16 data_offset;
    cb_u16 data_segment;
    cb_u16 draw_x;
    cb_u16 draw_y;
    cb_u16 extent_width;
    cb_u16 extent_height;
    cb_u8 tail[16];
} bloodprg_entity_record;

typedef struct bloodprg_sprite_source_extent {
    cb_u16 width;
    cb_u16 height;
} bloodprg_sprite_source_extent;

extern volatile bloodprg_entity_record bloodprg_entity_table[]; /* GS:0x6212 */

void CB_FAR entity_flag_state_transition(cb_u16 object_id); /* 0x0299:0x1241 */
void CB_FAR sprite_slot_position_update(cb_u16 object_id,
        cb_u16 draw_x,
        cb_u16 draw_y); /* 0x0299:0x127D */
/* 0x0299:0x133D; source_extent normalizes the inherited SS:BP+4 context. */
void CB_FAR sprite_slot_extent_update(cb_u16 object_id,
        cb_u16 width,
        cb_u16 height,
        const volatile bloodprg_sprite_source_extent CB_FAR *source_extent);

void CB_FAR entity_record_setter(cb_u16 entity_id,
        const volatile void CB_FAR *resource,
        cb_u16 draw_x,
        cb_u16 draw_y,
        cb_u16 frame_index); /* 0x0299:0x11BE */

#if defined(__WATCOMC__)
#pragma aux entity_flag_state_transition parm [ax]
#pragma aux sprite_slot_position_update parm [ax] [bx] [cx]
#pragma aux sprite_slot_extent_update parm [ax] [cx] [dx] [es si]
#endif

#endif
