#include <dos.h>

#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#endif

#include "../include/bloodprg_graphics.h"

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemset)
#endif

#define BLOODPRG_PLANAR_COUNT 4u

void CB_FAR gfx_clipped_span_fill(
        cb_u8 color,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width)
{
    volatile cb_u8 CB_FAR *pixel;
    volatile cb_u8 CB_FAR *plane_origin;
    cb_i16 clip_delta;
    cb_i16 clip_right;
    cb_u16 clip_amount;
    cb_u16 clipped_width;
    cb_u16 count;
    cb_u16 middle_count;
    cb_u16 original_width;
    cb_u16 plane_number;
    cb_u16 row_offset;
    cb_u16 span_end;
    cb_u8 map_mask;
    cb_u8 partial_count;
    cb_u8 start_plane;

    clipped_width = width;
    if ((cb_i16)clipped_width <= 0
            || (cb_i16)y < (cb_i16)graphics_band_top_row
            || (cb_i16)y >= (cb_i16)graphics_band_bottom_row) {
        return;
    }

    /* Preserve the shipped SUB/Jcc clipping behavior across 16-bit overflow. */
    clip_delta = (cb_i16)(x - (cb_u16)graphics_clip_left);
    if (clip_delta < 0) {
        clip_amount = (cb_u16)(0u - (cb_u16)clip_delta);
        original_width = clipped_width;
        clipped_width = (cb_u16)(clipped_width - clip_amount);
        if ((cb_i16)original_width <= (cb_i16)clip_amount) {
            return;
        }
        x = (cb_u16)graphics_clip_left;
    }

    clip_right = graphics_clip_right;
    span_end = (cb_u16)(x + clipped_width);
    clip_delta = (cb_i16)(span_end - (cb_u16)clip_right);
    if ((cb_i16)span_end >= clip_right) {
        original_width = clipped_width;
        clipped_width = (cb_u16)(clipped_width - (cb_u16)clip_delta);
        if ((cb_i16)original_width <= clip_delta) {
            return;
        }
    }

    row_offset = (cb_u16)(y << 4);
    row_offset += (cb_u16)(y << 6);
    start_plane = (cb_u8)x & (BLOODPRG_PLANAR_COUNT - 1u);
    pixel = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);

    if ((graphics_span_remap_enabled & 1u) != 0) {
        map_mask = (cb_u8)(0x11u << start_plane);
        plane_origin = pixel;
        partial_count = (cb_u8)(clipped_width & 3u);
        middle_count = clipped_width >> 2;

        for (plane_number = 0;
                plane_number < BLOODPRG_PLANAR_COUNT;
                ++plane_number) {
            outportb(0x03c5u, map_mask);
            pixel = plane_origin;
            count = middle_count;
            if (plane_number < partial_count) {
                ++count;
            }
            while (count-- != 0u) {
                *pixel = graphics_span_remap_table[*pixel];
                ++pixel;
            }

            if ((map_mask & 0x80u) != 0) {
                ++plane_origin;
            }
            map_mask = (cb_u8)((map_mask << 1) | (map_mask >> 7));
        }
        return;
    }

    if (start_plane != 0u) {
        partial_count = (cb_u8)(BLOODPRG_PLANAR_COUNT - start_plane);
        if (clipped_width < partial_count) {
            map_mask = (cb_u8)(
                    ((1u << clipped_width) - 1u) << start_plane);
            outportb(0x03c5u, map_mask);
            *pixel = color;
            return;
        }

        map_mask = (cb_u8)(0x0fu << start_plane);
        outportb(0x03c5u, map_mask);
        *pixel++ = color;
        clipped_width = (cb_u16)(clipped_width - partial_count);
    }

    middle_count = clipped_width >> 2;
    outportb(0x03c5u, 0x0fu);
    _fmemset((void CB_FAR *)pixel, color, middle_count);
    pixel += middle_count;

    partial_count = (cb_u8)(clipped_width & 3u);
    if (partial_count != 0u) {
        map_mask = (cb_u8)(0x0fu >> (BLOODPRG_PLANAR_COUNT - partial_count));
        outportb(0x03c5u, map_mask);
        *pixel = color;
    }

}
