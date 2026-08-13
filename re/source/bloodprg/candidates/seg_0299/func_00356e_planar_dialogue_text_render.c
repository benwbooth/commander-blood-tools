#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PLANAR_ROW_BYTES 80u
#define BLOODPRG_PLANAR_COUNT 4u
#define BLOODPRG_PLANAR_DIALOGUE_CLIP_HEIGHT 10u
#define BLOODPRG_MAIN_FONT_HEIGHT 8u

void CB_FAR planar_dialogue_text_render(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color)
{
    volatile cb_u8 CB_FAR *row_origin;
    volatile cb_u8 CB_FAR *glyph_origin;
    cb_u16 advance_word;
    cb_u16 glyph_offset;
    cb_u16 plane_number;
    cb_u16 row;
    cb_u16 row_offset;
    cb_u8 advance;
    cb_u8 character;
    cb_u8 glyph_index;
    cb_u8 map_mask;
    cb_u8 plane;
    cb_u8 row_bits;

#if defined(__WATCOMC__)
    _asm push eax;
    _asm push ds;
    _asm push es;
#endif

    main_font_draw_width = 0;
    if (y > graphics_band_bottom_row
            || (cb_i16)y
                    <= (cb_i16)(graphics_band_top_row
                            - BLOODPRG_PLANAR_DIALOGUE_CLIP_HEIGHT)) {
        goto restore_registers;
    }

    row_offset = (cb_u16)(y << 4);
    row_offset += (cb_u16)(y << 6);
    plane = (cb_u8)x & 3u;
    glyph_origin = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    while ((character = *text++) != 0) {
        glyph_index = main_font_draw_character_map[character];
        advance = main_font_draw_advance_table[glyph_index];
        glyph_offset = (cb_u16)glyph_index * BLOODPRG_MAIN_FONT_HEIGHT;
        map_mask = (cb_u8)(0x11u << plane);
        outportb(0x03c5u, map_mask);

        for (plane_number = 0;
                plane_number < BLOODPRG_PLANAR_COUNT;
                ++plane_number) {
            row_origin = glyph_origin;
            if ((cb_u16)plane + plane_number >= BLOODPRG_PLANAR_COUNT) {
                ++row_origin;
            }
            for (row = 0; row < BLOODPRG_MAIN_FONT_HEIGHT; ++row) {
                row_bits = main_font_draw_glyphs[glyph_offset + row];
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

        main_font_draw_width = (cb_u16)(main_font_draw_width
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
    _asm pop eax;
#endif
}
