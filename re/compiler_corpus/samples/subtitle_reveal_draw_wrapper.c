#include <dos.h>

#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#endif

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_draw_framebuffer;
extern const u8 __based(__segname("GAME_DATA"))
        subtitle_console_character_map[256];
extern const u8 __based(__segname("GAME_DATA"))
        subtitle_console_glyphs[];
extern volatile u16 __based(__segname("GAME_DATA"))
        subtitle_reveal_cursor;

#define PLANAR_ROW_BYTES 80u
#define PLANAR_COUNT 4u
#define FONT_HEIGHT 8u

void far subtitle_reveal_draw_wrapper(
        const u8 *line,
        u16 x,
        u16 y);

#pragma aux subtitle_reveal_draw_wrapper \
        parm [si] [bx] [dx] modify exact []

void far subtitle_reveal_draw_wrapper(
        const u8 *line,
        u16 x,
        u16 y)
{
    const u8 *line_cursor;
    const u8 *line_end;
    volatile u8 far *glyph_origin;
    volatile u8 far *row_origin;
    u16 glyph_offset;
    u16 line_length;
    u16 plane_number;
    u16 reveal_distance;
    u16 row;
    u16 row_offset;
    u8 character;
    u8 characters_remaining;
    u8 color;
    u8 glyph_index;
    u8 map_mask;
    u8 plane;
    u8 row_bits;

    _asm push eax;
    _asm push ds;
    _asm push es;

    line_end = line;
    line_length = 0;
    while (*line_end != '\r') {
        ++line_end;
        ++line_length;
    }

    row_offset = (u16)(y << 4);
    row_offset += (u16)(y << 6);
    plane = (u8)x & 3u;
    glyph_origin = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    map_mask = (u8)(0x11u << plane);

    line_cursor = line;
    characters_remaining = (u8)line_length;
    do {
        outportb(0x03c5u, map_mask);
        reveal_distance = (u16)(subtitle_reveal_cursor - (u16)line_cursor);
        if ((i16)reveal_distance < 0) {
            goto restore_registers;
        }
        if ((u8)reveal_distance == 0u) {
            color = 0xffu;
        } else if ((u8)reveal_distance == 1u) {
            color = 0xfeu;
        } else {
            color = 0xfdu;
        }

        character = *line_cursor++;
        glyph_index = subtitle_console_character_map[character];
        if ((i8)glyph_index >= 0) {
            glyph_offset = (u16)glyph_index * FONT_HEIGHT;
            for (plane_number = 0; plane_number < PLANAR_COUNT; ++plane_number) {
                row_origin = glyph_origin;
                if ((u16)plane + plane_number >= PLANAR_COUNT) {
                    ++row_origin;
                }
                for (row = 0; row < FONT_HEIGHT; ++row) {
                    row_bits = subtitle_console_glyphs[glyph_offset + row];
                    if ((row_bits & (u8)(0x80u >> plane_number)) != 0) {
                        row_origin[0] = color;
                    }
                    if ((row_bits & (u8)(0x08u >> plane_number)) != 0) {
                        row_origin[1] = color;
                    }
                    row_origin += PLANAR_ROW_BYTES;
                }

                map_mask = (u8)((map_mask << 1) | (map_mask >> 7));
                outportb(0x03c5u, map_mask);
            }
        }

        glyph_origin += 2u;
    } while (--characters_remaining != 0u);

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop eax;
}
