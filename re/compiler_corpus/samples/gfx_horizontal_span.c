typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_display_buffer;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_left;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_right;
extern volatile u16 __based(__segname("GAME_DATA")) graphics_band_top_row;
extern volatile u16 __based(__segname("GAME_DATA")) graphics_band_bottom_row;
extern volatile u8 __based(__segname("GAME_DATA"))
        graphics_span_remap_enabled;
extern const u8 __based(__segname("GAME_DATA"))
        graphics_span_remap_table[256];

#define SCREEN_WIDTH 320u

void far gfx_horizontal_span(u8 color, u16 x, u16 y, u16 width)
{
    volatile u8 far *pixel;
    i16 clip_delta;
    i16 clip_right;
    u16 clip_amount;
    u16 clipped_width;
    u16 original_width;
    u16 row_offset;
    u16 span_end;
    u16 count;

    _asm push ax;
    _asm push ds;
    _asm push es;

    clipped_width = width;
    if ((i16)clipped_width <= 0
            || (i16)y < (i16)graphics_band_top_row
            || (i16)y >= (i16)graphics_band_bottom_row) {
        goto restore_registers;
    }

    /* Keep the original SUB/Jcc operand comparisons across 16-bit overflow. */
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

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
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
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}

#pragma aux gfx_horizontal_span parm [ax] [bx] [cx] [dx] modify exact []
