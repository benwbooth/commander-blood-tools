#ifndef BLOODPRG_ENTITY_H
#define BLOODPRG_ENTITY_H

#include "bloodprg_common.h"

#define BLOODPRG_ENTITY_STATE0_FLAG 0x0001u
#define BLOODPRG_ENTITY_DIRTY_FLAG 0x0002u
#define BLOODPRG_ENTITY_ACTIVE_FLAG 0x0080u
#define BLOODPRG_ENTITY_ACTIVE_OR_STATE0_MASK 0x0081u

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

extern volatile bloodprg_entity_record bloodprg_entity_table[]; /* GS:0x6212 */

#endif
