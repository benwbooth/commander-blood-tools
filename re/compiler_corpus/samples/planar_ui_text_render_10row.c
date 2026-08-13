#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_screen_buffer;
extern volatile u16 __based(__segname("GAME_DATA"))
        graphics_band_top_row;
extern volatile u16 __based(__segname("GAME_DATA"))
        graphics_band_bottom_row;
extern const u8 __based(__segname("GAME_DATA"))
        square_caps_draw_character_map[256];
extern const u8 __based(__segname("GAME_DATA"))
        square_caps_draw_advance_table[];
extern const u8 __based(__segname("GAME_DATA"))
        square_caps_draw_glyphs[];
extern volatile u16 __based(__segname("GAME_DATA"))
        square_caps_draw_width;

#define PLANAR_ROW_BYTES 80u
#define PLANAR_COUNT 4u
#define FONT_HEIGHT 10u
#define GLYPH_BYTES 20u

void far planar_ui_text_render_10row(
        const u8 far *text,
        u16 x,
        u16 y,
        u8 color)
{
    volatile u8 far *glyph_origin;
    volatile u8 far *pixel;
    volatile u8 far *row_origin;
    u16 advance_word;
    u16 glyph_offset;
    u16 plane_number;
    u16 row;
    u16 row_bits;
    u16 row_offset;
    u8 advance;
    u8 character;
    u8 glyph_index;
    u8 map_mask;
    u8 plane;

    _asm push ax;
    _asm push ds;
    _asm push es;

    square_caps_draw_width = 0;
    if (y > graphics_band_bottom_row
            || (i16)y <= (i16)(graphics_band_top_row - FONT_HEIGHT)) {
        goto restore_registers;
    }

    row_offset = (u16)(y << 4);
    row_offset += (u16)(y << 6);
    plane = (u8)x & 3u;
    glyph_origin = graphics_screen_buffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    while ((character = *text++) != 0) {
        glyph_index = square_caps_draw_character_map[character];
        advance = square_caps_draw_advance_table[glyph_index];
        glyph_offset = (u16)glyph_index * GLYPH_BYTES;

        for (plane_number = 0; plane_number < PLANAR_COUNT; ++plane_number) {
            map_mask = (u8)(0x11u << (u8)((plane + plane_number) & 3u));
            outportb(0x03c5u, map_mask);
            row_origin = glyph_origin;
            if ((u16)plane + plane_number >= PLANAR_COUNT) {
                ++row_origin;
            }

            for (row = 0; row < FONT_HEIGHT; ++row) {
                row_bits = (u16)((u16)square_caps_draw_glyphs[glyph_offset]
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
                row_origin += PLANAR_ROW_BYTES;
            }
            glyph_offset -= GLYPH_BYTES;
        }

        square_caps_draw_width = (u16)(square_caps_draw_width
                + (i16)(i8)advance);
        advance_word = (u16)(i16)(i8)advance;
        advance_word = (u16)((advance_word & 0xff00u)
                | (u8)((u8)advance_word + plane));
        plane = (u8)advance_word & 3u;
        glyph_origin += advance_word >> 2;
    }

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}

#pragma aux planar_ui_text_render_10row \
        parm [ds si] [bx] [dx] [ax] modify exact []
