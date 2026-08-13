#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u

void CB_FAR gfx_horizontal_span(
        cb_u8 color,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width)
{
    volatile cb_u8 CB_FAR *pixel;
    cb_i16 clip_delta;
    cb_i16 clip_right;
    cb_u16 clip_amount;
    cb_u16 clipped_width;
    cb_u16 original_width;
    cb_u16 row_offset;
    cb_u16 span_end;
    cb_u16 count;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push ds;
    _asm push es;
#endif

    clipped_width = width;
    if ((cb_i16)clipped_width <= 0
            || (cb_i16)y < (cb_i16)graphics_band_top_row
            || (cb_i16)y >= (cb_i16)graphics_band_bottom_row) {
        goto restore_registers;
    }

    /* Keep the original SUB/Jcc operand comparisons across 16-bit overflow. */
    clip_delta = (cb_i16)(x - (cb_u16)graphics_clip_left);
    if (clip_delta < 0) {
        clip_amount = (cb_u16)(0u - (cb_u16)clip_delta);
        original_width = clipped_width;
        clipped_width = (cb_u16)(clipped_width - clip_amount);
        if ((cb_i16)original_width <= (cb_i16)clip_amount) {
            goto restore_registers;
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
            goto restore_registers;
        }
    }

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;

    count = clipped_width;
    if ((graphics_span_remap_enabled & 1u) != 0) {
        do {
            *pixel = graphics_span_remap_table[*pixel];
            ++pixel;
        } while (--count != 0);
    } else {
        do {
            *pixel++ = color;
        } while (--count != 0);
    }

restore_registers:
#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
#endif
}
