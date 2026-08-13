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
        main_font_draw_character_map[256];
extern const u8 __based(__segname("GAME_DATA"))
        main_font_draw_advance_table[];
extern const u8 __based(__segname("GAME_DATA"))
        main_font_draw_glyphs[];
extern volatile u16 __based(__segname("GAME_DATA"))
        main_font_draw_width;

#define SCREEN_WIDTH 320u
#define FONT_HEIGHT 8u
#define SPACE_ADVANCE 6u

void far main_font_text_draw_display(
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
    u8 row_bits;
    u8 row;
    u8 character;
    u8 glyph_index;
    i8 advance;

    _asm push ax;
    _asm push ds;
    _asm push es;

    main_font_draw_width = 0;
    if (y > graphics_band_bottom_row
            || (i16)y <= (i16)(graphics_band_top_row - FONT_HEIGHT)) {
        goto restore_registers;
    }

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
    glyph_origin = graphics_display_buffer + row_offset + x;

    while ((character = *text++) != 0) {
        if (character == ' ') {
            glyph_origin += SPACE_ADVANCE;
            continue;
        }

        glyph_index = main_font_draw_character_map[character];
        if ((glyph_index & 0x80u) != 0) {
            continue;
        }

        advance = (i8)main_font_draw_advance_table[glyph_index];
        glyph_offset = (u16)glyph_index * FONT_HEIGHT;
        row_origin = glyph_origin;

        for (row = 0; row < FONT_HEIGHT; ++row) {
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
            row_origin += SCREEN_WIDTH;
        }

        glyph_origin += advance;
        main_font_draw_width =
                (u16)(main_font_draw_width + advance);
    }

restore_registers:
    _asm pop es;
    _asm pop ds;
    _asm pop ax;
}

#pragma aux main_font_text_draw_display \
        parm [ds si] [bx] [dx] [ax] modify exact []
