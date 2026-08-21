#include "../include/bloodprg_entity.h"

void CB_NEAR sprite_blit_raw_transparent(
        volatile bloodprg_entity_record *record)
{
    const volatile bloodprg_sprite_frame CB_FAR *frame;
    const volatile cb_u8 CB_FAR *source;
    volatile cb_u8 CB_FAR *destination;
    volatile cb_u8 CB_FAR *row_destination;
    volatile cb_u8 CB_NEAR *remap;
    cb_i16 sprite_left;
    cb_i16 sprite_top;
    cb_i16 sprite_right;
    cb_i16 sprite_bottom;
    cb_i16 destination_x;
    cb_i16 destination_y;
    cb_u16 draw_width;
    cb_u16 draw_height;
    cb_u16 frame_stride;
    cb_u16 clipped;
    cb_u16 source_row_skip;
    cb_u16 rows;
    cb_u16 columns;
    cb_u8 pixel;
    cb_u8 flip_x;
    cb_u8 flip_y;

    frame = record->frame;
    flip_x = (cb_u8)((record->flags & 0x0020u) != 0u);
    flip_y = (cb_u8)((record->flags & 0x0040u) != 0u);
    sprite_top = (cb_i16)(record->draw_y + (cb_u16)frame->y_offset);
    sprite_right = (cb_i16)(record->draw_x + record->extent_width +
            (cb_u16)frame->x_offset);
    sprite_bottom = (cb_i16)(record->draw_y + record->extent_height +
            (cb_u16)frame->y_offset);
    frame_stride = frame->stride;
    source = (const volatile cb_u8 CB_FAR *)frame;
    draw_width = record->extent_width;
    draw_height = record->extent_height;
    destination_y = sprite_top;

    if (sprite_top < (cb_i16)record->dirty_rect.top) {
        clipped = (cb_u16)((cb_i16)record->dirty_rect.top - sprite_top);
        draw_height = (cb_u16)(draw_height - clipped);
        if ((flip_y & 1u) == 0u) {
            source += (cb_u16)(clipped * frame_stride);
        }
        destination_y = (cb_i16)record->dirty_rect.top;
    }
    if (sprite_bottom >= (cb_i16)record->dirty_rect.bottom) {
        clipped = (cb_u16)(sprite_bottom -
                (cb_i16)record->dirty_rect.bottom);
        draw_height = (cb_u16)(draw_height - clipped);
        if ((flip_y & 1u) != 0u) {
            source += (cb_u16)(clipped * frame_stride);
        }
    }

    /* The original reloads +4 through the vertically adjusted source cursor. */
    sprite_left = (cb_i16)(record->draw_x + (cb_u16)
            ((const volatile bloodprg_sprite_frame CB_FAR *)source)->x_offset);
    destination_x = sprite_left;
    if (sprite_left < (cb_i16)record->dirty_rect.left) {
        clipped = (cb_u16)((cb_i16)record->dirty_rect.left - sprite_left);
        draw_width = (cb_u16)(draw_width - clipped);
        if ((flip_x & 1u) == 0u) {
            source += clipped;
        }
        destination_x = (cb_i16)record->dirty_rect.left;
    }
    if (sprite_right >= (cb_i16)record->dirty_rect.right) {
        clipped = (cb_u16)(sprite_right -
                (cb_i16)record->dirty_rect.right);
        draw_width = (cb_u16)(draw_width - clipped);
        if ((flip_x & 1u) != 0u) {
            source += clipped;
        }
    }

    switch ((record->flags >> 8) & 3u) {
    case 0u:
        remap = 0;
        break;
    case 1u:
        remap = &bloodprg_sprite_remap_5f11[0];
        break;
    default:
        remap = &bloodprg_sprite_remap_6011[0];
        break;
    }
    bloodprg_selected_sprite_remap = remap;

    if ((flip_y & 1u) != 0u) {
        destination_y = (cb_i16)(destination_y + draw_height - 1u);
    }
    if (flip_x != 0u) {
        destination_x = (cb_i16)(destination_x + draw_width - 1u);
    }
    source += 8u;
    row_destination = bloodprg_display_buffer +
            (cb_u16)((cb_u16)destination_y * 320u +
                    (cb_u16)destination_x);
    source_row_skip = (cb_u16)(frame_stride - draw_width);
    rows = draw_height;
    do {
        destination = row_destination;
        columns = draw_width;
        do {
            pixel = *source++;
            if (pixel != 0u) {
                if (remap != 0) {
                    *destination = remap[*destination];
                } else {
                    *destination = pixel;
                }
            }
            if (flip_x != 0u) {
                --destination;
            } else {
                ++destination;
            }
            --columns;
        } while (columns != 0u);

        source += source_row_skip;
        if (flip_y != 0u) {
            row_destination -= 320u;
        } else {
            row_destination += 320u;
        }
        --rows;
    } while (rows != 0u);
}
