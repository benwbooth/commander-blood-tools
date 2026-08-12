#include "../include/bloodprg_entity.h"

void CB_NEAR sprite_blit_scaled_transparent(
        volatile bloodprg_entity_record *record)
{
    const volatile bloodprg_sprite_frame CB_FAR *frame;
    const volatile cb_u8 CB_FAR *pixels;
    const volatile cb_u8 CB_FAR *source_row;
    volatile cb_u8 CB_FAR *destination;
    cb_u32 x_step;
    cb_u32 y_step;
    cb_u32 x_start;
    cb_u32 x_position;
    cb_u32 y_position;
    cb_u16 source_width;
    cb_u16 source_height;
    cb_i16 destination_x;
    cb_i16 destination_y;
    cb_i16 draw_width;
    cb_i16 draw_height;
    cb_u16 clipped;
    cb_u16 rows;
    cb_u16 columns;
    cb_u8 pixel;

    frame = record->frame;
    source_width = frame->stride;
    if (record->extent_width == 0u) {
        return;
    }
    x_step = ((cb_u32)source_width << 16) / record->extent_width;

    source_height = frame->height;
    if (record->extent_height == 0u) {
        return;
    }
    y_step = ((cb_u32)source_height << 16) / record->extent_height;
    x_start = 0u;
    y_position = 0u;
    destination_x = (cb_i16)record->draw_x;
    destination_y = (cb_i16)record->draw_y;
    draw_width = (cb_i16)record->extent_width;
    draw_height = (cb_i16)record->extent_height;

    if (destination_y < (cb_i16)record->dirty_rect.top) {
        clipped = (cb_u16)((cb_i16)record->dirty_rect.top - destination_y);
        draw_height = (cb_i16)(draw_height - (cb_i16)clipped);
        y_position = (cb_u32)clipped * y_step;
        destination_y = (cb_i16)record->dirty_rect.top;
    }
    if ((cb_i16)(record->draw_y + record->extent_height) >=
            (cb_i16)record->dirty_rect.bottom) {
        clipped = (cb_u16)((cb_i16)(record->draw_y +
                record->extent_height) - (cb_i16)record->dirty_rect.bottom);
        draw_height = (cb_i16)(draw_height - (cb_i16)clipped);
    }

    if (destination_x < (cb_i16)record->dirty_rect.left) {
        clipped = (cb_u16)((cb_i16)record->dirty_rect.left - destination_x);
        draw_width = (cb_i16)(draw_width - (cb_i16)clipped);
        x_start = (cb_u32)clipped * x_step;
        destination_x = (cb_i16)record->dirty_rect.left;
    }
    if ((cb_i16)(record->draw_x + record->extent_width) >=
            (cb_i16)record->dirty_rect.right) {
        clipped = (cb_u16)((cb_i16)(record->draw_x +
                record->extent_width) - (cb_i16)record->dirty_rect.right);
        draw_width = (cb_i16)(draw_width - (cb_i16)clipped);
    }

    if (draw_width <= 0 || draw_height <= 0) {
        return;
    }

    pixels = (const volatile cb_u8 CB_FAR *)frame + 8u;
    destination = bloodprg_display_buffer +
            (cb_u16)((cb_u16)destination_y * 320u +
                    (cb_u16)destination_x);
    rows = (cb_u16)draw_height;
    do {
        source_row = pixels +
                (cb_u16)((y_position >> 16) * source_width);
        x_position = x_start;
        columns = (cb_u16)draw_width;
        do {
            pixel = source_row[(cb_u16)(x_position >> 16)];
            if (pixel != 0u) {
                *destination = pixel;
            }
            ++destination;
            x_position += x_step;
            --columns;
        } while (columns != 0u);

        destination += (cb_u16)(320u - (cb_u16)draw_width);
        y_position += y_step;
        --rows;
    } while (rows != 0u);
}
