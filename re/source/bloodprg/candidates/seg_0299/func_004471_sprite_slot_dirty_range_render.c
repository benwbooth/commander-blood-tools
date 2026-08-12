#include "../include/bloodprg_entity.h"

void CB_FAR sprite_slot_dirty_range_render(cb_u16 first_object_id,
        cb_u16 last_object_id)
{
    volatile bloodprg_entity_record *record;
    volatile bloodprg_dirty_rect *dirty_rect;
    cb_u16 remaining;
    cb_u16 flags;
    cb_u16 slot_right;
    cb_u16 slot_bottom;

    if ((cb_i16)bloodprg_dirty_rect_list[0].left < 0) {
        return;
    }

    remaining = (cb_u16)(last_object_id - first_object_id + 1u);
    record = &bloodprg_entity_table[(cb_u16)(last_object_id + 1u)];
    do {
        --record;
        flags = record->flags;
        if ((flags & BLOODPRG_ENTITY_STATE0_FLAG) != 0u) {
            bloodprg_selected_sprite_blitter =
                    bloodprg_sprite_blitter_table[(flags >> 2) & 7u];
            bloodprg_sprite_flip_x =
                    (cb_u8)((flags & 0x0020u) != 0u);
            bloodprg_sprite_flip_y =
                    (cb_u8)((flags & 0x0040u) != 0u);

            slot_right = (cb_u16)(record->draw_x + record->extent_width);
            slot_bottom = (cb_u16)(record->draw_y + record->extent_height);
            dirty_rect = &bloodprg_dirty_rect_list[0];
            do {
                record->dirty_rect = *dirty_rect;
                if ((cb_i16)record->draw_x < (cb_i16)dirty_rect->right &&
                        (cb_i16)record->draw_y < (cb_i16)dirty_rect->bottom &&
                        (cb_i16)slot_right > (cb_i16)dirty_rect->left &&
                        (cb_i16)slot_bottom > (cb_i16)dirty_rect->top) {
                    bloodprg_selected_sprite_blitter(record);
                }
                ++dirty_rect;
            } while ((cb_i16)dirty_rect->left >= 0);
        }

        record->flags = (cb_u16)(record->flags & ~BLOODPRG_ENTITY_DIRTY_FLAG);
        --remaining;
    } while (remaining != 0u);
}
