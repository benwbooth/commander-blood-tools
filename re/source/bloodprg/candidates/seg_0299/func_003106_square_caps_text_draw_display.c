#include "../include/bloodprg_graphics.h"

#define BLOODPRG_SCREEN_WIDTH 320u
#define BLOODPRG_SQUARE_CAPS_HEIGHT 10u
#define BLOODPRG_SQUARE_CAPS_GLYPH_BYTES 20u

void CB_FAR square_caps_text_draw_display(
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
    cb_u16 row_bits;
    cb_u16 row;
    cb_u8 character;
    cb_u8 glyph_index;
    cb_u8 advance;
    cb_i16 advance_delta;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push ds;
    _asm push es;
#endif

    square_caps_draw_width = 0;
    if (y > graphics_band_bottom_row
            || (cb_i16)y
                    <= (cb_i16)(graphics_band_top_row
                            - BLOODPRG_SQUARE_CAPS_HEIGHT)) {
        goto restore_registers;
    }

    row_offset = (cb_u16)((y << 8) | (y >> 8));
    row_offset += (cb_u16)(y << 6);
    glyph_origin = graphics_display_buffer + row_offset + x;

    while ((character = *text++) != 0) {
        glyph_index = square_caps_draw_character_map[character];
        advance = square_caps_draw_advance_table[glyph_index];
        advance_delta = (cb_i16)(cb_i8)advance;
        glyph_offset =
                (cb_u16)glyph_index * BLOODPRG_SQUARE_CAPS_GLYPH_BYTES;
        row_origin = glyph_origin;

        for (row = 0; row < BLOODPRG_SQUARE_CAPS_HEIGHT; ++row) {
            row_bits = (cb_u16)((cb_u16)square_caps_draw_glyphs[glyph_offset]
                    << 8);
            row_bits |= square_caps_draw_glyphs[glyph_offset + 1u];
            glyph_offset += 2;
            pixel = row_origin;
            while (row_bits != 0) {
                if ((row_bits & 0x8000u) != 0) {
                    *pixel = color;
                }
                row_bits <<= 1;
                if (row_bits != 0) {
                    ++pixel;
                }
            }
            row_origin += BLOODPRG_SCREEN_WIDTH;
        }

        glyph_origin += advance_delta;
        square_caps_draw_width =
                (cb_u16)(square_caps_draw_width + advance_delta);
    }

restore_registers:
#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
#endif
}
