#include "../include/bloodprg_entity.h"

void CB_FAR sprite_slot_position_update(cb_u16 object_id, cb_u16 draw_x, cb_u16 draw_y)
{
    volatile bloodprg_entity_record *record;
    cb_u16 flags;
    cb_u8 low_flags;

    record = &bloodprg_entity_table[object_id];
    flags = record->flags;
    low_flags = (cb_u8)flags;

    if ((low_flags & BLOODPRG_ENTITY_ACTIVE_OR_STATE0_MASK) != 0u) {
        if (record->draw_x != draw_x) {
            low_flags = (cb_u8)(low_flags | BLOODPRG_ENTITY_DIRTY_FLAG);
            record->draw_x = draw_x;
        }
        if (record->draw_y != draw_y) {
            low_flags = (cb_u8)(low_flags | BLOODPRG_ENTITY_DIRTY_FLAG);
            record->draw_y = draw_y;
        }
        flags = (cb_u16)((flags & 0xff00u) | low_flags);
    }

    record->flags = flags;
}
