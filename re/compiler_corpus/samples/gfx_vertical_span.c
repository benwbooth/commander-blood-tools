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

void far gfx_vertical_span(u8 color, u16 x, u16 y, u16 height)
{
    volatile u8 far *pixel;
    i16 clip_bottom;
    i16 clip_delta;
    u16 clip_amount;
    u16 clipped_height;
    u16 original_height;
    u16 row_offset;
    u16 span_end;
    u16 count;
    u8 remapped;

    _asm push ax;
    _asm push ds;
    _asm push es;
    _asm push bx;

    remapped = 0;
    clipped_height = height;
    if ((i16)clipped_height <= 0
            || (i16)x < graphics_clip_left
            || (i16)x >= graphics_clip_right) {
        goto restore_registers;
    }

    /* Keep the original SUB/Jcc operand comparisons across 16-bit overflow. */
    clip_delta = (i16)(y - (u16)graphics_band_top_row);
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

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
    pixel = graphics_display_buffer + row_offset + x;

    count = clipped_height;
    if ((graphics_span_remap_enabled & 1u) != 0) {
        remapped = 1;
        do {
            *pixel = graphics_span_remap_table[*pixel];
            pixel += SCREEN_WIDTH;
        } while (--count != 0);
    } else {
        do {
            *pixel = color;
            pixel += SCREEN_WIDTH;
        } while (--count != 0);
    }

restore_registers:
    _asm pop bx;
    if (remapped != 0) {
        _asm mov bx, 0x5f11;
    }
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}

#pragma aux gfx_vertical_span parm [ax] [bx] [cx] [dx] modify exact [bx]
