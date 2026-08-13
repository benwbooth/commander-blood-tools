#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_display_buffer;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_left;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_right;
extern volatile u16 __based(__segname("GAME_DATA")) graphics_band_top_row;
extern volatile u16 __based(__segname("GAME_DATA")) graphics_band_bottom_row;

#define SCREEN_WIDTH 320u

void far framebuffer_rect_fill(
        u8 color, u16 x, u16 y, u16 width, u16 height);
#pragma aux framebuffer_rect_fill parm caller [ax] [bx] [cx] [dx] modify exact []

void far framebuffer_rect_fill(
        u8 color, u16 x, u16 y, u16 width, u16 height)
{
    volatile u8 far *pixel;
    volatile u8 far *row_pixel;
    volatile u32 far *dword_pixel;
    i16 clip_delta;
    i32 clipped_extent;
    u32 packed_color;
    u16 clipped_height;
    u16 clipped_width;
    u16 count;
    u16 dword_count;
    u16 row_offset;
    u16 row_skip;
    u16 span_end;
    u8 leading_count;
    u8 remainder;
    u8 rows_remaining;
    u8 trailing_count;

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
        clipped_extent = (i32)(i16)clipped_width + clip_delta;
        if (clipped_extent <= 0) {
            goto restore_registers;
        }
        clipped_width = (u16)clipped_extent;
        x = (u16)graphics_clip_left;
    }

    span_end = (u16)(x + clipped_width);
    clip_delta = (i16)(span_end - (u16)graphics_clip_right);
    if ((i16)span_end > graphics_clip_right) {
        clipped_extent = (i32)(i16)clipped_width - clip_delta;
        if (clipped_extent <= 0) {
            goto restore_registers;
        }
        clipped_width = (u16)clipped_extent;
    }

    clip_delta = (i16)(y - graphics_band_top_row);
    if (clip_delta < 0) {
        clipped_extent = (i32)(i16)clipped_height + clip_delta;
        if (clipped_extent <= 0) {
            goto restore_registers;
        }
        clipped_height = (u16)clipped_extent;
        y = graphics_band_top_row;
    }

    span_end = (u16)(y + clipped_height);
    clip_delta = (i16)(span_end - graphics_band_bottom_row);
    if ((i16)span_end > (i16)graphics_band_bottom_row) {
        clipped_extent = (i32)(i16)clipped_height - clip_delta;
        if (clipped_extent <= 0) {
            goto restore_registers;
        }
        clipped_height = (u16)clipped_extent;
    }

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;
    row_skip = (u16)(SCREEN_WIDTH - clipped_width);

    leading_count = (u8)(FP_OFF(pixel) & 3u);
    remainder = (u8)(clipped_width & 3u);
    dword_count = clipped_width >> 2;
    trailing_count = remainder;

    if (dword_count == 0u) {
        leading_count = 0u;
    } else {
        if (remainder < leading_count) {
            --dword_count;
        }
        trailing_count = (u8)((remainder - leading_count) & 3u);
        if (dword_count == 0u) {
            trailing_count = (u8)(trailing_count + leading_count);
            leading_count = 0u;
        }
    }

    if (dword_count != 0u) {
        packed_color = (u16)(color | ((u16)color << 8));
        packed_color |= packed_color << 16;
    }
    rows_remaining = (u8)clipped_height;

    do {
        row_pixel = pixel;

        count = leading_count;
        while (count-- != 0u) {
            *row_pixel++ = color;
        }

        if (dword_count != 0u) {
            dword_pixel = (volatile u32 far *)row_pixel;
            count = dword_count;
            while (count-- != 0u) {
                *dword_pixel++ = packed_color;
            }
            row_pixel = (volatile u8 far *)dword_pixel;
        }

        count = trailing_count;
        while (count-- != 0u) {
            *row_pixel++ = color;
        }

        pixel = row_pixel + row_skip;
    } while (--rows_remaining != 0u);

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}
