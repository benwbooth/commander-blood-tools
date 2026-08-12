#include "../include/bloodprg_entity.h"

void CB_FAR sprite_slot_range_mark_dirty(cb_u16 first_object_id,
        cb_u16 last_object_id)
{
    volatile bloodprg_entity_record *record;
    cb_u16 remaining;
    bloodprg_entity_flags flags;

    remaining = (cb_u16)(last_object_id - first_object_id + 1u);
    record = &bloodprg_entity_table[first_object_id];

    while (remaining != 0u) {
        flags.word = record->flags;
        if ((flags.bytes.low & BLOODPRG_ENTITY_ACTIVE_FLAG) != 0u) {
            flags.bytes.low = (cb_u8)((flags.bytes.low & 0x7eu) |
                    BLOODPRG_ENTITY_DIRTY_FLAG);
            record->flags = flags.word;
        }
        ++record;
        --remaining;
    }
}
