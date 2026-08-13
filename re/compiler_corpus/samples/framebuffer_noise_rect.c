#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_display_buffer;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_left;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_right;
extern volatile u16 __based(__segname("GAME_DATA")) graphics_band_top_row;

extern u16 far blood_prng_next(u16 modulus);
#pragma aux blood_prng_next parm [ax] value [ax] modify exact [ax]

#define SCREEN_WIDTH 320u
#define NOISE_COLOR 0xefu
#define NOISE_WORD_BITS 16u

void far framebuffer_noise_rect(
        u16 mode, u16 x, u16 y, u16 width, u16 height);
#pragma aux framebuffer_noise_rect parm caller [ax] [bx] [cx] [dx] modify exact []

void far framebuffer_noise_rect(
        u16 mode, u16 x, u16 y, u16 width, u16 height)
{
    volatile u8 far *pixel;
    i16 clip_delta;
    u8 bits_remaining;
    u16 clipped_height;
    u16 clipped_width;
    u16 columns;
    u16 old_pattern;
    u16 pattern;
    u16 row_offset;
    u16 row_skip;
    u16 rows;
    u8 current_color;
    u8 next_bit;
    u8 rotation_bit;
    u8 sparse_color;

    _asm push ax;
    _asm push ds;
    _asm push es;

    clipped_width = width;
    _asm mov ax, word ptr [bp];
    _asm mov clipped_height, ax;

    if ((i16)clipped_width <= 0 || (i16)clipped_height <= 0) {
        goto restore_registers;
    }

    clip_delta = (i16)(x - (u16)graphics_clip_left);
    if (clip_delta < 0) {
        clipped_width = (u16)(clipped_width + (u16)clip_delta);
        if ((i16)clipped_width <= 0) {
            goto restore_registers;
        }
        x = (u16)graphics_clip_left;
    }

    clip_delta = (i16)(
            (u16)(x + clipped_width) - (u16)graphics_clip_right);
    if (clip_delta >= 0) {
        clipped_width = (u16)(clipped_width - (u16)clip_delta);
        if ((i16)clipped_width <= 0) {
            goto restore_registers;
        }
    }

    clip_delta = (i16)(y - graphics_band_top_row);
    if (clip_delta < 0) {
        clipped_height = (u16)(clipped_height + (u16)clip_delta);
        if ((i16)clipped_height <= 0) {
            goto restore_registers;
        }
        y = graphics_band_top_row;
    }

    clip_delta = (i16)(
            (u16)(y + clipped_height) - (u16)graphics_clip_right);
    if (clip_delta >= 0) {
        clipped_height = (u16)(clipped_height - (u16)clip_delta);
        if ((i16)clipped_height <= 0) {
            goto restore_registers;
        }
    }

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;
    row_skip = (u16)(SCREEN_WIDTH - clipped_width);

    pattern = blood_prng_next(0xffffu);
    rotation_bit = 1u;
    bits_remaining = NOISE_WORD_BITS;
    rows = clipped_height;

    if (mode == 1u || mode == 2u) {
        sparse_color = mode == 1u ? NOISE_COLOR : 0u;
        do {
            columns = clipped_width;
            do {
                next_bit = (u8)(pattern >> 15);
                pattern = (u16)((pattern << 1) | rotation_bit);
                rotation_bit = next_bit;
                if (next_bit != 0u) {
                    *pixel = sparse_color;
                }
                ++pixel;

                if (--bits_remaining == 0u) {
                    old_pattern = pattern;
                    pattern = (u16)(
                            (pattern << 4)
                            | ((u16)rotation_bit << 3)
                            | (pattern >> 13));
                    pattern ^= old_pattern;
                    rotation_bit = 0u;
                    bits_remaining = NOISE_WORD_BITS;
                }
            } while (--columns != 0u);

            rotation_bit = (u8)(
                    row_skip > (u16)(0xffffu - FP_OFF(pixel)));
            pixel += row_skip;
        } while (--rows != 0u);
    } else {
        current_color = 0u;
        do {
            columns = clipped_width;
            do {
                next_bit = (u8)(pattern >> 15);
                pattern = (u16)((pattern << 1) | rotation_bit);
                rotation_bit = next_bit;
                if (next_bit != 0u) {
                    current_color ^= NOISE_COLOR;
                }
                *pixel++ = current_color;

                if (--bits_remaining == 0u) {
                    old_pattern = pattern;
                    pattern = (u16)(
                            (pattern << 3)
                            | ((u16)rotation_bit << 2)
                            | (pattern >> 14));
                    pattern ^= old_pattern;
                    rotation_bit = 0u;
                    bits_remaining = NOISE_WORD_BITS;
                }
            } while (--columns != 0u);

            rotation_bit = (u8)(
                    row_skip > (u16)(0xffffu - FP_OFF(pixel)));
            pixel += row_skip;
        } while (--rows != 0u);
    }

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}
