#include <conio.h>
#include <dos.h>

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

#define PLANAR_ROW_BYTES 80u

void far gfx_clipped_planar_vertical_span(
        u8 color, u16 x, u16 y, u16 height);

#pragma aux gfx_clipped_planar_vertical_span \
        parm [ax] [bx] [cx] [dx] modify exact []

void far gfx_clipped_planar_vertical_span(
        u8 color, u16 x, u16 y, u16 height)
{
    volatile u8 far *pixel;
    i16 clip_bottom;
    i16 clip_delta;
    u16 clip_amount;
    u16 clipped_height;
    u16 count;
    u16 original_height;
    u16 row_offset;
    u16 span_end;
    u16 sequencer_word;

    _asm push ax;
    _asm push ds;
    _asm push es;

    clipped_height = height;
    if ((i16)clipped_height <= 0
            || (i16)x < graphics_clip_left
            || (i16)x >= graphics_clip_right) {
        goto restore_registers;
    }

    clip_delta = (i16)(y - graphics_band_top_row);
    if (clip_delta < 0) {
        clip_amount = (u16)(0u - (u16)clip_delta);
        original_height = clipped_height;
        clipped_height = (u16)(clipped_height - clip_amount);
        if ((i16)original_height <= (i16)clip_amount) {
            goto restore_registers;
        }
        y = graphics_band_top_row;
    }

    clip_bottom = (i16)graphics_band_bottom_row;
    span_end = (u16)(y + clipped_height);
    clip_delta = (i16)(span_end - (u16)clip_bottom);
    if ((i16)span_end > clip_bottom) {
        original_height = clipped_height;
        clipped_height = (u16)(clipped_height - (u16)clip_delta);
        if ((i16)original_height <= clip_delta) {
            goto restore_registers;
        }
    }

    row_offset = (u16)(y << 4);
    row_offset += (u16)(y << 6);
    pixel = graphics_draw_framebuffer + row_offset + (x >> 2);

    sequencer_word = (u16)(2u | ((u16)(1u << (x & 3u)) << 8));
    outpw(0x03c4u, sequencer_word);

    count = clipped_height;
    if ((graphics_span_remap_enabled & 1u) != 0) {
        do {
            *pixel = graphics_span_remap_table[*pixel];
            pixel += PLANAR_ROW_BYTES;
        } while (--count != 0u);
    } else {
        do {
            *pixel = color;
            pixel += PLANAR_ROW_BYTES;
        } while (--count != 0u);
    }

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}
