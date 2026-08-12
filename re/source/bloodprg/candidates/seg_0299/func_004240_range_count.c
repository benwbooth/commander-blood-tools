#include "../include/bloodprg_entity.h"

void CB_FAR range_count(cb_u16 first_object_id, cb_u16 last_object_id)
{
    volatile bloodprg_entity_record *record;
    cb_u16 remaining;
    cb_u16 flags;
    cb_u8 low_flags;

    remaining = (cb_u16)(last_object_id - first_object_id + 1u);
    record = &bloodprg_entity_table[first_object_id];

    while (remaining != 0u) {
        flags = record->flags;
        low_flags = (cb_u8)flags;

        if ((low_flags & BLOODPRG_ENTITY_ACTIVE_FLAG) != 0u) {
            low_flags = (cb_u8)((low_flags & 0x7eu) | BLOODPRG_ENTITY_DIRTY_FLAG);
            record->flags = (cb_u16)((flags & 0xff00u) | low_flags);
        }

        ++record;
        --remaining;
    }
}
