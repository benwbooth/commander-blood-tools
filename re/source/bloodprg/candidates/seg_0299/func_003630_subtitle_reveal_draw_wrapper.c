#include <dos.h>

#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#endif

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PLANAR_ROW_BYTES 80u
#define BLOODPRG_PLANAR_COUNT 4u
#define BLOODPRG_SUBTITLE_FONT_HEIGHT 8u

void CB_FAR subtitle_reveal_draw_wrapper(
        const cb_u8 CB_NEAR *line,
        cb_u16 x,
        cb_u16 y)
{
    const cb_u8 CB_NEAR *line_cursor;
    const cb_u8 CB_NEAR *line_end;
    volatile cb_u8 CB_FAR *glyph_origin;
    volatile cb_u8 CB_FAR *row_origin;
    cb_u16 glyph_offset;
    cb_u16 line_length;
    cb_u16 plane_number;
    cb_u16 reveal_distance;
    cb_u16 row;
    cb_u16 row_offset;
    cb_u8 character;
    cb_u8 characters_remaining;
    cb_u8 color;
    cb_u8 glyph_index;
    cb_u8 map_mask;
    cb_u8 plane;
    cb_u8 row_bits;

    line_end = line;
    line_length = 0;
    while (*line_end != '\r') {
        ++line_end;
        ++line_length;
    }

    row_offset = (cb_u16)(y << 4);
    row_offset += (cb_u16)(y << 6);
    plane = (cb_u8)x & 3u;
    glyph_origin = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    map_mask = (cb_u8)(0x11u << plane);

    line_cursor = line;
    characters_remaining = (cb_u8)line_length;
    do {
        outportb(0x03c5u, map_mask);
        reveal_distance = (cb_u16)(subtitle_reveal_cursor
                - (cb_u16)line_cursor);
        if ((cb_i16)reveal_distance < 0) {
            return;
        }
        if ((cb_u8)reveal_distance == 0u) {
            color = 0xffu;
        } else if ((cb_u8)reveal_distance == 1u) {
            color = 0xfeu;
        } else {
            color = 0xfdu;
        }

        character = *line_cursor++;
        glyph_index = subtitle_console_character_map[character];
        if ((cb_i8)glyph_index >= 0) {
            glyph_offset = (cb_u16)glyph_index
                    * BLOODPRG_SUBTITLE_FONT_HEIGHT;
            for (plane_number = 0;
                    plane_number < BLOODPRG_PLANAR_COUNT;
                    ++plane_number) {
                row_origin = glyph_origin;
                if ((cb_u16)plane + plane_number >= BLOODPRG_PLANAR_COUNT) {
                    ++row_origin;
                }
                for (row = 0; row < BLOODPRG_SUBTITLE_FONT_HEIGHT; ++row) {
                    row_bits = subtitle_console_glyphs[glyph_offset + row];
                    if ((row_bits & (cb_u8)(0x80u >> plane_number)) != 0) {
                        row_origin[0] = color;
                    }
                    if ((row_bits & (cb_u8)(0x08u >> plane_number)) != 0) {
                        row_origin[1] = color;
                    }
                    row_origin += BLOODPRG_PLANAR_ROW_BYTES;
                }

                map_mask = (cb_u8)((map_mask << 1) | (map_mask >> 7));
                outportb(0x03c5u, map_mask);
            }
        }

        glyph_origin += 2u;
    } while (--characters_remaining != 0u);

}
