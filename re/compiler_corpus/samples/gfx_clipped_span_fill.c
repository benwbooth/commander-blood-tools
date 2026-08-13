#if defined(__WATCOMC__)
#include <conio.h>
#include <string.h>
#define outportb outp
#pragma intrinsic(_fmemset)
#else
#include <dos.h>
#endif

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_draw_framebuffer;
extern volatile u16 __based(__segname("GAME_DATA"))
        graphics_band_top_row;
extern volatile u16 __based(__segname("GAME_DATA"))
        graphics_band_bottom_row;
extern volatile i16 __based(__segname("GAME_DATA"))
        graphics_clip_left;
extern volatile i16 __based(__segname("GAME_DATA"))
        graphics_clip_right;
extern volatile u8 __based(__segname("GAME_DATA"))
        graphics_span_remap_enabled;
extern const u8 __based(__segname("GAME_DATA"))
        graphics_span_remap_table[256];

#define PLANAR_COUNT 4u

void far gfx_clipped_span_fill(u8 color, u16 x, u16 y, u16 width);

#pragma aux gfx_clipped_span_fill parm [ax] [bx] [cx] [dx] modify exact []

void far gfx_clipped_span_fill(u8 color, u16 x, u16 y, u16 width)
{
    volatile u8 far *pixel;
    volatile u8 far *plane_origin;
    i16 clip_delta;
    i16 clip_right;
    u16 clip_amount;
    u16 clipped_width;
    u16 count;
    u16 middle_count;
    u16 original_width;
    u16 plane_number;
    u16 row_offset;
    u16 span_end;
    u8 map_mask;
    u8 partial_count;
    u8 start_plane;

    _asm push ax;
    _asm push ds;
    _asm push es;

    clipped_width = width;
    if ((i16)clipped_width <= 0
            || (i16)y < (i16)graphics_band_top_row
            || (i16)y >= (i16)graphics_band_bottom_row) {
        goto restore_registers;
    }

    clip_delta = (i16)(x - (u16)graphics_clip_left);
    if (clip_delta < 0) {
        clip_amount = (u16)(0u - (u16)clip_delta);
        original_width = clipped_width;
        clipped_width = (u16)(clipped_width - clip_amount);
        if ((i16)original_width <= (i16)clip_amount) {
            goto restore_registers;
        }
        x = (u16)graphics_clip_left;
    }

    clip_right = graphics_clip_right;
    span_end = (u16)(x + clipped_width);
    clip_delta = (i16)(span_end - (u16)clip_right);
    if ((i16)span_end >= clip_right) {
        original_width = clipped_width;
        clipped_width = (u16)(clipped_width - (u16)clip_delta);
        if ((i16)original_width <= clip_delta) {
            goto restore_registers;
        }
    }

    row_offset = (u16)(y << 4);
    row_offset += (u16)(y << 6);
    start_plane = (u8)x & (PLANAR_COUNT - 1u);
    pixel = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);

    if ((graphics_span_remap_enabled & 1u) != 0) {
        map_mask = (u8)(0x11u << start_plane);
        plane_origin = pixel;
        partial_count = (u8)(clipped_width & 3u);
        middle_count = clipped_width >> 2;

        for (plane_number = 0; plane_number < PLANAR_COUNT; ++plane_number) {
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
            map_mask = (u8)((map_mask << 1) | (map_mask >> 7));
        }
        goto restore_registers;
    }

    if (start_plane != 0u) {
        partial_count = (u8)(PLANAR_COUNT - start_plane);
        if (clipped_width < partial_count) {
            map_mask = (u8)(((1u << clipped_width) - 1u) << start_plane);
            outportb(0x03c5u, map_mask);
            *pixel = color;
            goto restore_registers;
        }

        map_mask = (u8)(0x0fu << start_plane);
        outportb(0x03c5u, map_mask);
        *pixel++ = color;
        clipped_width = (u16)(clipped_width - partial_count);
    }

    middle_count = clipped_width >> 2;
    outportb(0x03c5u, 0x0fu);
    _fmemset((void far *)pixel, color, middle_count);
    pixel += middle_count;

    partial_count = (u8)(clipped_width & 3u);
    if (partial_count != 0u) {
        map_mask = (u8)(0x0fu >> (PLANAR_COUNT - partial_count));
        outportb(0x03c5u, map_mask);
        *pixel = color;
    }

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}
