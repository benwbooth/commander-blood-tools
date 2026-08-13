typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_display_buffer;
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

#define SCREEN_WIDTH 320u
#define FONT_HEIGHT 10u
#define GLYPH_BYTES 20u

void far square_caps_text_draw_display(
        const u8 far *text,
        u16 x,
        u16 y,
        u8 color)
{
    volatile u8 far *glyph_origin;
    volatile u8 far *pixel;
    volatile u8 far *row_origin;
    u16 glyph_offset;
    u16 row_offset;
    u16 row_bits;
    u16 row;
    u8 character;
    u8 glyph_index;
    u8 advance;
    i16 advance_delta;

    _asm push ax;
    _asm push ds;
    _asm push es;

    square_caps_draw_width = 0;
    if (y > graphics_band_bottom_row
            || (i16)y <= (i16)(graphics_band_top_row - FONT_HEIGHT)) {
        goto restore_registers;
    }

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
    glyph_origin = graphics_display_buffer + row_offset + x;

    while ((character = *text++) != 0) {
        glyph_index = square_caps_draw_character_map[character];
        advance = square_caps_draw_advance_table[glyph_index];
        advance_delta = (i16)(i8)advance;
        glyph_offset = (u16)glyph_index * GLYPH_BYTES;
        row_origin = glyph_origin;

        for (row = 0; row < FONT_HEIGHT; ++row) {
            row_bits = (u16)((u16)square_caps_draw_glyphs[glyph_offset]
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
            row_origin += SCREEN_WIDTH;
        }

        glyph_origin += advance_delta;
        square_caps_draw_width =
                (u16)(square_caps_draw_width + advance_delta);
    }

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}

#pragma aux square_caps_text_draw_display \
        parm [ds si] [bx] [dx] [ax] modify exact []
