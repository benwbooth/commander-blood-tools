#include <dos.h>

#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#endif

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PLANAR_ROW_BYTES 80u
#define BLOODPRG_PLANAR_COUNT 4u
#define BLOODPRG_SMALL_FONT_HEIGHT 5u

void CB_FAR small_text_render(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color)
{
    volatile cb_u8 CB_FAR *glyph_origin;
    volatile cb_u8 CB_FAR *row_origin;
    cb_u16 glyph_offset;
    cb_u16 plane_number;
    cb_u16 row;
    cb_u16 row_offset;
    cb_u8 character;
    cb_u8 glyph_index;
    cb_u8 map_mask;
    cb_u8 plane;
    cb_u8 row_bits;

    row_offset = (cb_u16)(y << 4);
    row_offset += (cb_u16)(y << 6);
    plane = (cb_u8)x & 3u;
    glyph_origin = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    map_mask = (cb_u8)(0x11u << plane);
    for (;;) {
        outportb(0x03c5u, map_mask);
        character = *text++;
        if (character == 0) {
            break;
        }

        glyph_index = small_font_character_map[character];
        if ((cb_i8)glyph_index >= 0) {
            glyph_offset = (cb_u16)glyph_index * BLOODPRG_SMALL_FONT_HEIGHT;
            for (plane_number = 0;
                    plane_number < BLOODPRG_PLANAR_COUNT;
                    ++plane_number) {
                row_origin = glyph_origin;
                if ((cb_u16)plane + plane_number >= BLOODPRG_PLANAR_COUNT) {
                    ++row_origin;
                }
                for (row = 0; row < BLOODPRG_SMALL_FONT_HEIGHT; ++row) {
                    row_bits = small_font_glyphs[glyph_offset + row];
                    if ((row_bits & (cb_u8)(0x80u >> plane_number)) != 0) {
                        row_origin[0] = color;
                    }
                    row_origin += BLOODPRG_PLANAR_ROW_BYTES;
                }

                map_mask = (cb_u8)((map_mask << 1) | (map_mask >> 7));
                outportb(0x03c5u, map_mask);
            }
        }

        ++glyph_origin;
    }
}
