#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u

void CB_FAR framebuffer_rect_palette_remap(
        const cb_u8 CB_FAR *remap_table,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 height)
{
    volatile cb_u8 CB_FAR *pixel;
    cb_i16 clip_delta;
    cb_u16 clipped_height;
    cb_u16 clipped_width;
    cb_u16 count;
    cb_u16 row_offset;
    cb_u16 row_skip;
    cb_u16 rows;

    clipped_width = width;
    clipped_height = height;
    if ((cb_i16)clipped_width <= 0 || (cb_i16)clipped_height <= 0) {
        return;
    }

    clip_delta = (cb_i16)(x - (cb_u16)graphics_clip_left);
    if (clip_delta < 0) {
        clipped_width = (cb_u16)(clipped_width + (cb_u16)clip_delta);
        if ((cb_i16)clipped_width <= 0) {
            return;
        }
        x = (cb_u16)graphics_clip_left;
    }

    clip_delta = (cb_i16)(
            (cb_u16)(x + clipped_width) - (cb_u16)graphics_clip_right);
    if (clip_delta >= 0) {
        clipped_width = (cb_u16)(clipped_width - (cb_u16)clip_delta);
        if ((cb_i16)clipped_width <= 0) {
            return;
        }
    }

    clip_delta = (cb_i16)(y - graphics_band_top_row);
    if (clip_delta < 0) {
        clipped_height = (cb_u16)(clipped_height + (cb_u16)clip_delta);
        if ((cb_i16)clipped_height <= 0) {
            return;
        }
        y = graphics_band_top_row;
    }

    /* The shipped routine uses the X-right bound here, not the Y-bottom bound. */
    clip_delta = (cb_i16)(
            (cb_u16)(y + clipped_height) - (cb_u16)graphics_clip_right);
    if (clip_delta >= 0) {
        clipped_height = (cb_u16)(clipped_height - (cb_u16)clip_delta);
        if ((cb_i16)clipped_height <= 0) {
            return;
        }
    }

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;
    row_skip = (cb_u16)(BLOODPRG_SCREEN_WIDTH - clipped_width);

    rows = clipped_height;
    do {
        count = clipped_width;
        do {
            *pixel = remap_table[*pixel];
            ++pixel;
        } while (--count != 0);
        pixel += row_skip;
    } while (--rows != 0);
}
