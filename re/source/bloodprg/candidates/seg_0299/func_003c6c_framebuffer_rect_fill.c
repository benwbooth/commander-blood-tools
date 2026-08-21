#include <dos.h>

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u

void CB_FAR framebuffer_rect_fill(
        cb_u8 color,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 height)
{
    volatile cb_u8 CB_FAR *pixel;
    volatile cb_u8 CB_FAR *row_pixel;
    volatile cb_u32 CB_FAR *dword_pixel;
    cb_i16 clip_delta;
    cb_i32 clipped_extent;
    cb_u32 packed_color;
    cb_u16 clipped_height;
    cb_u16 clipped_width;
    cb_u16 count;
    cb_u16 dword_count;
    cb_u16 row_offset;
    cb_u16 row_skip;
    cb_u16 span_end;
    cb_u8 leading_count;
    cb_u8 remainder;
    cb_u8 rows_remaining;
    cb_u8 trailing_count;

    clipped_width = width;
    clipped_height = height;

    if ((cb_i16)clipped_width <= 0 || (cb_i16)clipped_height <= 0) {
        return;
    }

    clip_delta = (cb_i16)(x - (cb_u16)graphics_clip_left);
    if (clip_delta < 0) {
        clipped_extent = (cb_i32)(cb_i16)clipped_width + clip_delta;
        if (clipped_extent <= 0) {
            return;
        }
        clipped_width = (cb_u16)clipped_extent;
        x = (cb_u16)graphics_clip_left;
    }

    span_end = (cb_u16)(x + clipped_width);
    clip_delta = (cb_i16)(span_end - (cb_u16)graphics_clip_right);
    if ((cb_i16)span_end > graphics_clip_right) {
        clipped_extent = (cb_i32)(cb_i16)clipped_width - clip_delta;
        if (clipped_extent <= 0) {
            return;
        }
        clipped_width = (cb_u16)clipped_extent;
    }

    clip_delta = (cb_i16)(y - graphics_band_top_row);
    if (clip_delta < 0) {
        clipped_extent = (cb_i32)(cb_i16)clipped_height + clip_delta;
        if (clipped_extent <= 0) {
            return;
        }
        clipped_height = (cb_u16)clipped_extent;
        y = graphics_band_top_row;
    }

    span_end = (cb_u16)(y + clipped_height);
    clip_delta = (cb_i16)(span_end - graphics_band_bottom_row);
    if ((cb_i16)span_end > (cb_i16)graphics_band_bottom_row) {
        clipped_extent = (cb_i32)(cb_i16)clipped_height - clip_delta;
        if (clipped_extent <= 0) {
            return;
        }
        clipped_height = (cb_u16)clipped_extent;
    }

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;
    row_skip = (cb_u16)(BLOODPRG_SCREEN_WIDTH - clipped_width);

    leading_count = (cb_u8)(FP_OFF(pixel) & 3u);
    remainder = (cb_u8)(clipped_width & 3u);
    dword_count = clipped_width >> 2;
    trailing_count = remainder;

    if (dword_count == 0u) {
        leading_count = 0u;
    } else {
        if (remainder < leading_count) {
            --dword_count;
        }
        trailing_count = (cb_u8)((remainder - leading_count) & 3u);
        if (dword_count == 0u) {
            trailing_count = (cb_u8)(trailing_count + leading_count);
            leading_count = 0u;
        }
    }

    if (dword_count != 0u) {
        packed_color = (cb_u16)(color | ((cb_u16)color << 8));
        packed_color |= packed_color << 16;
    }
    rows_remaining = (cb_u8)clipped_height;

    do {
        row_pixel = pixel;

        count = leading_count;
        while (count-- != 0u) {
            *row_pixel++ = color;
        }

        if (dword_count != 0u) {
            dword_pixel = (volatile cb_u32 CB_FAR *)row_pixel;
            count = dword_count;
            while (count-- != 0u) {
                *dword_pixel++ = packed_color;
            }
            row_pixel = (volatile cb_u8 CB_FAR *)dword_pixel;
        }

        count = trailing_count;
        while (count-- != 0u) {
            *row_pixel++ = color;
        }

        pixel = row_pixel + row_skip;
    } while (--rows_remaining != 0u);
}
