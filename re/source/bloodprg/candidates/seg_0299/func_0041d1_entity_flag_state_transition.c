#include "../include/bloodprg_entity.h"

void CB_FAR entity_flag_state_transition(cb_u16 object_id)
{
    volatile bloodprg_entity_record *record;
    bloodprg_entity_flags flags;

    record = &bloodprg_entity_table[object_id];
    flags.word = record->flags;

    if ((flags.bytes.low & BLOODPRG_ENTITY_ACTIVE_FLAG) != 0u &&
        (flags.bytes.low & BLOODPRG_ENTITY_STATE0_FLAG) != 0u) {
        flags.bytes.low = (cb_u8)((flags.bytes.low & 0xfeu) |
                BLOODPRG_ENTITY_DIRTY_FLAG);
    }

    record->flags = flags.word;
}
