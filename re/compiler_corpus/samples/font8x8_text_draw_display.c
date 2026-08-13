typedef unsigned char u8;
typedef unsigned int u16;

typedef volatile u8 far *graphics_buffer_ptr;
typedef const u8 far *font_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_display_buffer;
extern font_ptr __based(__segname("GAME_DATA")) bios_font_8x8;

#define SCREEN_WIDTH 320u
#define FONT_WIDTH 8u
#define FONT_HEIGHT 8u

const u8 far *far font8x8_text_draw_display(
        const u8 far *text,
        u16 x,
        u16 y,
        u16 color_and_limit)
{
    volatile u8 far *glyph_origin;
    volatile u8 far *pixel;
    const u8 far *glyph;
    u16 row;
    u16 column;
    u16 row_offset;
    u8 character;
    u8 bits;
    u8 color;
    u8 max_characters;

    row_offset = (u16)((y << 8) | (y >> 8));
    row_offset += (u16)(y << 6);
    glyph_origin = graphics_display_buffer + row_offset + x;
    color = (u8)color_and_limit;
    max_characters = (u8)(color_and_limit >> 8);
    do {
        character = *text;
        if (character == 0) {
            break;
        }

        glyph = bios_font_8x8 + (u16)character * FONT_HEIGHT;
        pixel = glyph_origin;
        for (row = 0; row < FONT_HEIGHT; ++row) {
            bits = *glyph++;
            for (column = 0; column < FONT_WIDTH; ++column) {
                if ((bits & 0x80u) != 0) {
                    *pixel = color;
                }
                bits <<= 1;
                ++pixel;
            }
            pixel += SCREEN_WIDTH - FONT_WIDTH;
        }

        ++text;
        glyph_origin += FONT_WIDTH;
    } while (--max_characters != 0);

    return text;
}

#pragma aux font8x8_text_draw_display \
        parm [ds si] [ax] [bx] [dx] value [ds si] modify exact [si]
