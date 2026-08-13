#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u
#define BLOODPRG_MAIN_FONT_HEIGHT 8u
#define BLOODPRG_SPACE_ADVANCE 6u

void CB_FAR main_font_text_draw_display(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color)
{
    volatile cb_u8 CB_FAR *glyph_origin;
    volatile cb_u8 CB_FAR *pixel;
    volatile cb_u8 CB_FAR *row_origin;
    cb_u16 glyph_offset;
    cb_u16 row_offset;
    cb_u8 row_bits;
    cb_u8 row;
    cb_u8 character;
    cb_u8 glyph_index;
    cb_i8 advance;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push ds;
    _asm push es;
#endif

    main_font_draw_width = 0;
    if (y > graphics_band_bottom_row
            || (cb_i16)y
                    <= (cb_i16)(graphics_band_top_row
                            - BLOODPRG_MAIN_FONT_HEIGHT)) {
        goto restore_registers;
    }

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    glyph_origin = graphics_display_buffer + row_offset + x;

    while ((character = *text++) != 0) {
        if (character == ' ') {
            glyph_origin += BLOODPRG_SPACE_ADVANCE;
            continue;
        }

        glyph_index = main_font_draw_character_map[character];
        if ((glyph_index & 0x80u) != 0) {
            continue;
        }

        advance = (cb_i8)main_font_draw_advance_table[glyph_index];
        glyph_offset = (cb_u16)glyph_index * BLOODPRG_MAIN_FONT_HEIGHT;
        row_origin = glyph_origin;

        for (row = 0; row < BLOODPRG_MAIN_FONT_HEIGHT; ++row) {
            row_bits = main_font_draw_glyphs[glyph_offset++];
            pixel = row_origin;
            for (;;) {
                if ((row_bits & 0x80u) != 0) {
                    *pixel = color;
                }
                row_bits <<= 1;
                if (row_bits == 0) {
                    break;
                }
                ++pixel;
            }
            row_origin += BLOODPRG_SCREEN_WIDTH;
        }

        glyph_origin += advance;
        main_font_draw_width =
                (cb_u16)(main_font_draw_width + advance);
    }

restore_registers:
#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
#endif
}
