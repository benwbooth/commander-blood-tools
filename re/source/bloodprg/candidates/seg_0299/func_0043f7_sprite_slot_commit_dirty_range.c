#include "../include/bloodprg_entity.h"

void CB_FAR sprite_slot_commit_dirty_range(cb_u16 first_object_id,
        cb_u16 last_object_id)
{
    volatile bloodprg_entity_record *record;
    cb_u16 remaining;
    bloodprg_entity_flags flags;

    if ((bloodprg_clip_snapshot_flags & 1u) != 0u) {
        bloodprg_dirty_rect_list[0] = bloodprg_clip_bounds;
        bloodprg_dirty_rect_list[1].left = 0xffffu;
        bloodprg_clip_snapshot_flags = 0;
        return;
    }

    remaining = (cb_u16)(last_object_id - first_object_id + 1u);
    record = &bloodprg_entity_table[first_object_id];
    while (remaining != 0u) {
        flags.word = record->flags;
        if ((flags.bytes.low & BLOODPRG_ENTITY_DIRTY_FLAG) != 0u &&
                (flags.bytes.low & BLOODPRG_ENTITY_STATE0_FLAG) != 0u) {
            record->committed_draw_x = record->draw_x;
            record->committed_draw_y = record->draw_y;
            record->committed_extent_width = record->extent_width;
            record->committed_extent_height = record->extent_height;
        }
        ++record;
        --remaining;
    }
}
