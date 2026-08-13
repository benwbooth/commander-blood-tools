typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_display_buffer;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_left;
extern volatile i16 __based(__segname("GAME_DATA")) graphics_clip_right;
extern volatile u16 __based(__segname("GAME_DATA")) graphics_band_top_row;

#define SCREEN_WIDTH 320u

void far framebuffer_rect_palette_remap(
        const u8 far *remap_table,
        u16 x,
        u16 y,
        u16 width,
        u16 height)
{
    volatile u8 far *pixel;
    i16 clip_delta;
    u16 clipped_height;
    u16 clipped_width;
    u16 count;
    u16 row_offset;
    u16 row_skip;
    u16 rows;

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

    rows = clipped_height;
    do {
        count = clipped_width;
        do {
            *pixel = remap_table[*pixel];
            ++pixel;
        } while (--count != 0);
        pixel += row_skip;
    } while (--rows != 0);

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}

#pragma aux framebuffer_rect_palette_remap \
        parm caller [ds si] [bx] [cx] [dx] modify exact []
