#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u

void CB_FAR gfx_vertical_span(
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
    cb_u16 original_height;
    cb_u16 row_offset;
    cb_u16 span_end;
    cb_u16 count;
    cb_u8 remapped;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push ds;
    _asm push es;
    _asm push bx;
#endif

    remapped = 0;
    clipped_height = height;
    if ((cb_i16)clipped_height <= 0
            || (cb_i16)x < graphics_clip_left
            || (cb_i16)x >= graphics_clip_right) {
        goto restore_registers;
    }

    /* Keep the original SUB/Jcc operand comparisons across 16-bit overflow. */
    clip_delta = (cb_i16)(y - (cb_u16)graphics_band_top_row);
    if (clip_delta < 0) {
        clip_amount = (cb_u16)(0u - (cb_u16)clip_delta);
        original_height = clipped_height;
        clipped_height = (cb_u16)(clipped_height - clip_amount);
        if ((cb_i16)original_height <= (cb_i16)clip_amount) {
            goto restore_registers;
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
            goto restore_registers;
        }
    }

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;

    count = clipped_height;
    if ((graphics_span_remap_enabled & 1u) != 0) {
        remapped = 1;
        do {
            *pixel = graphics_span_remap_table[*pixel];
            pixel += BLOODPRG_SCREEN_WIDTH;
        } while (--count != 0);
    } else {
        do {
            *pixel = color;
            pixel += BLOODPRG_SCREEN_WIDTH;
        } while (--count != 0);
    }

restore_registers:
#if defined(__WATCOMC__)
    _asm pop bx;
    if (remapped != 0) {
        _asm mov bx, 0x5f11;
    }
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
#endif
}
