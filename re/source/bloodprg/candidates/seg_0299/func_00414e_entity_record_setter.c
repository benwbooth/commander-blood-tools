#include <dos.h>

#include "../include/bloodprg_entity.h"

void CB_FAR entity_record_setter(
        cb_u16 entity_id,
        const volatile void CB_FAR *resource_data,
        cb_u16 draw_x,
        cb_u16 draw_y,
        cb_u16 frame_index)
{
    const volatile bloodprg_entity_resource CB_FAR *resource;
    const volatile bloodprg_sprite_frame CB_FAR *frame;
    volatile bloodprg_entity_record CB_GAME_DATA *record;
    cb_u32 packed_frame;
    cb_u16 selected_frame;
    cb_u16 frame_segment;
    cb_u16 frame_offset;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    selected_frame = frame_index;
#if defined(__WATCOMC__)
    /* BP carries the fifth register argument and is saved at the frame base. */
    _asm mov ax, word ptr [bp];
    _asm mov selected_frame, ax;
#endif

    resource = (const volatile bloodprg_entity_resource CB_FAR *)resource_data;
    record = &bloodprg_entity_table[entity_id];
    if ((cb_i16)selected_frame >= resource->frame_count) {
        goto restore_registers;
    }

    record->flags = (cb_u16)(
            (resource->flags & BLOODPRG_ENTITY_RESOURCE_FLAG)
            | BLOODPRG_ENTITY_ACTIVATE_FLAGS);

    packed_frame = resource->packed_frame_offsets[selected_frame];
    frame_segment = (cb_u16)(
            FP_SEG(resource) + (cb_u16)(packed_frame >> 4));
    frame_offset = (cb_u16)(
            FP_OFF(resource)
            + (cb_u16)sizeof(resource->flags)
            + (cb_u16)sizeof(resource->frame_count)
            + (cb_u16)(packed_frame & 0x0fu));
    frame = (const volatile bloodprg_sprite_frame CB_FAR *)MK_FP(
            frame_segment, frame_offset);
    record->frame = frame;

    record->extent_width = frame->stride;
    if (record->committed_extent_width == 0u) {
        record->committed_extent_width = record->extent_width;
    }
    record->extent_height = frame->height;
    if (record->committed_extent_height == 0u) {
        record->committed_extent_height = record->extent_height;
    }
    record->draw_x = draw_x;
    record->draw_y = draw_y;

restore_registers:
#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
    return;
}
