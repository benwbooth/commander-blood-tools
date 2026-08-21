#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_random.h"

#define BLOODPRG_SCREEN_WIDTH 320u
#define BLOODPRG_NOISE_COLOR 0xefu
#define BLOODPRG_NOISE_WORD_BITS 16u

void CB_FAR framebuffer_noise_rect(
        cb_u16 mode,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 height)
{
    volatile cb_u8 CB_FAR *pixel;
    cb_i16 clip_delta;
    cb_u8 bits_remaining;
    cb_u16 clipped_height;
    cb_u16 clipped_width;
    cb_u16 columns;
    cb_u16 old_pattern;
    cb_u16 pattern;
    cb_u16 row_offset;
    cb_u16 row_skip;
    cb_u16 rows;
    cb_u8 current_color;
    cb_u8 next_bit;
    cb_u8 rotation_bit;
    cb_u8 sparse_color;

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

    pattern = blood_prng_next(0xffffu);
    rotation_bit = 1u;
    bits_remaining = BLOODPRG_NOISE_WORD_BITS;
    rows = clipped_height;

    if (mode == 1u || mode == 2u) {
        sparse_color = mode == 1u ? BLOODPRG_NOISE_COLOR : 0u;
        do {
            columns = clipped_width;
            do {
                next_bit = (cb_u8)(pattern >> 15);
                pattern = (cb_u16)((pattern << 1) | rotation_bit);
                rotation_bit = next_bit;
                if (next_bit != 0u) {
                    *pixel = sparse_color;
                }
                ++pixel;

                if (--bits_remaining == 0u) {
                    old_pattern = pattern;
                    pattern = (cb_u16)(
                            (pattern << 4)
                            | ((cb_u16)rotation_bit << 3)
                            | (pattern >> 13));
                    pattern ^= old_pattern;
                    rotation_bit = 0u;
                    bits_remaining = BLOODPRG_NOISE_WORD_BITS;
                }
            } while (--columns != 0u);

            rotation_bit = (cb_u8)(
                    row_skip > (cb_u16)(0xffffu - FP_OFF(pixel)));
            pixel += row_skip;
        } while (--rows != 0u);
    } else {
        current_color = 0u;
        do {
            columns = clipped_width;
            do {
                next_bit = (cb_u8)(pattern >> 15);
                pattern = (cb_u16)((pattern << 1) | rotation_bit);
                rotation_bit = next_bit;
                if (next_bit != 0u) {
                    current_color ^= BLOODPRG_NOISE_COLOR;
                }
                /* Compiler-generated callers enter with DF clear. */
                *pixel++ = current_color;

                if (--bits_remaining == 0u) {
                    old_pattern = pattern;
                    pattern = (cb_u16)(
                            (pattern << 3)
                            | ((cb_u16)rotation_bit << 2)
                            | (pattern >> 14));
                    pattern ^= old_pattern;
                    rotation_bit = 0u;
                    bits_remaining = BLOODPRG_NOISE_WORD_BITS;
                }
            } while (--columns != 0u);

            rotation_bit = (cb_u8)(
                    row_skip > (cb_u16)(0xffffu - FP_OFF(pixel)));
            pixel += row_skip;
        } while (--rows != 0u);
    }
}
