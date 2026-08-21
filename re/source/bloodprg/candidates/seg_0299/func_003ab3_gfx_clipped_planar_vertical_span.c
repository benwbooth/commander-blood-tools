#include <conio.h>
#include <dos.h>

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PLANAR_ROW_BYTES 80u

void CB_FAR gfx_clipped_planar_vertical_span(
        cb_u8 color,
        cb_u16 x,
        cb_u16 y,
        cb_u16 height)
{
    volatile cb_u8 CB_FAR *pixel;
    cb_i16 clip_bottom;
    cb_i16 clip_delta;
    cb_u16 clip_amount;
    cb_u16 clipped_height;
    cb_u16 count;
    cb_u16 original_height;
    cb_u16 row_offset;
    cb_u16 span_end;
    cb_u16 sequencer_word;

    clipped_height = height;
    if ((cb_i16)clipped_height <= 0
            || (cb_i16)x < graphics_clip_left
            || (cb_i16)x >= graphics_clip_right) {
        return;
    }

    /* Preserve the shipped SUB/Jcc clipping behavior across 16-bit overflow. */
    clip_delta = (cb_i16)(y - graphics_band_top_row);
    if (clip_delta < 0) {
        clip_amount = (cb_u16)(0u - (cb_u16)clip_delta);
        original_height = clipped_height;
        clipped_height = (cb_u16)(clipped_height - clip_amount);
        if ((cb_i16)original_height <= (cb_i16)clip_amount) {
            return;
        }
        y = graphics_band_top_row;
    }

    clip_bottom = (cb_i16)graphics_band_bottom_row;
    span_end = (cb_u16)(y + clipped_height);
    clip_delta = (cb_i16)(span_end - (cb_u16)clip_bottom);
    if ((cb_i16)span_end > clip_bottom) {
        original_height = clipped_height;
        clipped_height = (cb_u16)(clipped_height - (cb_u16)clip_delta);
        if ((cb_i16)original_height <= clip_delta) {
            return;
        }
    }

    row_offset = (cb_u16)(y << 4);
    row_offset += (cb_u16)(y << 6);
    pixel = graphics_draw_framebuffer + row_offset + (x >> 2);

    sequencer_word = (cb_u16)(
            2u | ((cb_u16)(1u << (x & 3u)) << 8));
    outpw(0x03c4u, sequencer_word);

    count = clipped_height;
    if ((graphics_span_remap_enabled & 1u) != 0) {
        do {
            *pixel = graphics_span_remap_table[*pixel];
            pixel += BLOODPRG_PLANAR_ROW_BYTES;
        } while (--count != 0u);
    } else {
        do {
            *pixel = color;
            pixel += BLOODPRG_PLANAR_ROW_BYTES;
        } while (--count != 0u);
    }

}
