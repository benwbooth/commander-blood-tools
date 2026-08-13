#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PLANAR_ROW_BYTES 80u
#define BLOODPRG_PLANAR_COUNT 4u
#define BLOODPRG_SQUARE_CAPS_HEIGHT 10u
#define BLOODPRG_SQUARE_CAPS_GLYPH_BYTES 20u

void CB_FAR planar_ui_text_render_10row(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color)
{
    volatile cb_u8 CB_FAR *glyph_origin;
    volatile cb_u8 CB_FAR *pixel;
    volatile cb_u8 CB_FAR *row_origin;
    cb_u16 advance_word;
    cb_u16 glyph_offset;
    cb_u16 plane_number;
    cb_u16 row;
    cb_u16 row_bits;
    cb_u16 row_offset;
    cb_u8 advance;
    cb_u8 character;
    cb_u8 glyph_index;
    cb_u8 map_mask;
    cb_u8 plane;

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

    row_offset = (cb_u16)(y << 4);
    row_offset += (cb_u16)(y << 6);
    plane = (cb_u8)x & 3u;
    glyph_origin = graphics_screen_buffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    while ((character = *text++) != 0) {
        glyph_index = square_caps_draw_character_map[character];
        advance = square_caps_draw_advance_table[glyph_index];
        glyph_offset =
                (cb_u16)glyph_index * BLOODPRG_SQUARE_CAPS_GLYPH_BYTES;

        for (plane_number = 0;
                plane_number < BLOODPRG_PLANAR_COUNT;
                ++plane_number) {
            map_mask = (cb_u8)(0x11u
                    << (cb_u8)((plane + plane_number) & 3u));
            outportb(0x03c5u, map_mask);
            row_origin = glyph_origin;
            if ((cb_u16)plane + plane_number >= BLOODPRG_PLANAR_COUNT) {
                ++row_origin;
            }

            for (row = 0; row < BLOODPRG_SQUARE_CAPS_HEIGHT; ++row) {
                row_bits = (cb_u16)(
                        (cb_u16)square_caps_draw_glyphs[glyph_offset]
                                << 8);
                row_bits |= square_caps_draw_glyphs[glyph_offset + 1u];
                row_bits <<= plane_number;
                pixel = row_origin;
                while (row_bits != 0) {
                    if ((row_bits & 0x8000u) != 0) {
                        *pixel = color;
                    }
                    row_bits <<= 4;
                    if (row_bits != 0) {
                        ++pixel;
                    }
                }
                glyph_offset += 2;
                row_origin += BLOODPRG_PLANAR_ROW_BYTES;
            }
            glyph_offset -= BLOODPRG_SQUARE_CAPS_GLYPH_BYTES;
        }

        square_caps_draw_width = (cb_u16)(square_caps_draw_width
                + (cb_i16)(cb_i8)advance);

        advance_word = (cb_u16)(cb_i16)(cb_i8)advance;
        advance_word = (cb_u16)((advance_word & 0xff00u)
                | (cb_u8)((cb_u8)advance_word + plane));
        plane = (cb_u8)advance_word & 3u;
        glyph_origin += advance_word >> 2;
    }

restore_registers:
#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
#endif
}
