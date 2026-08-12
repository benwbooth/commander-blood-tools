#include "../include/bloodprg_entity.h"

void CB_FAR sprite_slot_extent_update(cb_u16 object_id,
        cb_u16 width,
        cb_u16 height,
        const volatile bloodprg_sprite_source_extent CB_FAR *source_extent)
{
    volatile bloodprg_entity_record *record;
    bloodprg_entity_flags flags;

    record = &bloodprg_entity_table[object_id];
    flags.word = record->flags;

    if ((flags.bytes.low & BLOODPRG_ENTITY_ACTIVE_OR_STATE0_MASK) != 0u) {
        if (width == source_extent->width && height == source_extent->height) {
            if ((flags.bytes.low & BLOODPRG_ENTITY_EXTENT_CHANGED_FLAG) != 0u) {
                flags.bytes.low = (cb_u8)(
                        (flags.bytes.low & ~BLOODPRG_ENTITY_EXTENT_CHANGED_FLAG) |
                        BLOODPRG_ENTITY_DIRTY_FLAG);
            }
        } else if (width != record->extent_width ||
                height != record->extent_height) {
            flags.bytes.low = (cb_u8)(flags.bytes.low |
                    BLOODPRG_ENTITY_EXTENT_CHANGED_FLAG |
                    BLOODPRG_ENTITY_DIRTY_FLAG);
            record->extent_width = width;
            record->extent_height = height;
        }
    }

    record->flags = flags.word;
}
