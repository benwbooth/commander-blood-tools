#include "../include/bloodprg_entity.h"

void CB_FAR sprite_slot_position_update(cb_u16 object_id, cb_u16 draw_x, cb_u16 draw_y)
{
    volatile bloodprg_entity_record *record;
    bloodprg_entity_flags flags;

    record = &bloodprg_entity_table[object_id];
    flags.word = record->flags;

    if ((flags.bytes.low & BLOODPRG_ENTITY_ACTIVE_OR_STATE0_MASK) != 0u) {
        if (record->draw_x != draw_x) {
            flags.bytes.low = (cb_u8)(flags.bytes.low |
                    BLOODPRG_ENTITY_DIRTY_FLAG);
            record->draw_x = draw_x;
        }
        if (record->draw_y != draw_y) {
            flags.bytes.low = (cb_u8)(flags.bytes.low |
                    BLOODPRG_ENTITY_DIRTY_FLAG);
            record->draw_y = draw_y;
        }
    }

    record->flags = flags.word;
}
