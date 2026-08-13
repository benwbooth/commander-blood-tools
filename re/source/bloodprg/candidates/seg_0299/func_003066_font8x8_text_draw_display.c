#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u
#define BLOODPRG_FONT_WIDTH 8u
#define BLOODPRG_FONT_HEIGHT 8u

const cb_u8 CB_FAR *CB_FAR font8x8_text_draw_display(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u16 color_and_limit)
{
    volatile cb_u8 CB_FAR *glyph_origin;
    volatile cb_u8 CB_FAR *pixel;
    const cb_u8 CB_FAR *glyph;
    cb_u16 row;
    cb_u16 column;
    cb_u16 row_offset;
    cb_u8 character;
    cb_u8 bits;
    cb_u8 color;
    cb_u8 max_characters;

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    glyph_origin = graphics_display_buffer + row_offset + x;
    color = (cb_u8)color_and_limit;
    max_characters = (cb_u8)(color_and_limit >> 8);
    do {
        character = *text;
        if (character == 0) {
            break;
        }

        glyph = bios_font_8x8 + (cb_u16)character * BLOODPRG_FONT_HEIGHT;
        pixel = glyph_origin;
        for (row = 0; row < BLOODPRG_FONT_HEIGHT; ++row) {
            bits = *glyph++;
            for (column = 0; column < BLOODPRG_FONT_WIDTH; ++column) {
                if ((bits & 0x80u) != 0) {
                    *pixel = color;
                }
                bits <<= 1;
                ++pixel;
            }
            pixel += BLOODPRG_SCREEN_WIDTH - BLOODPRG_FONT_WIDTH;
        }

        ++text;
        glyph_origin += BLOODPRG_FONT_WIDTH;
    } while (--max_characters != 0);

    return text;
}
