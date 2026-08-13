#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_draw_framebuffer;
extern const u8 __based(__segname("GAME_DATA"))
        small_font_character_map[256];
extern const u8 __based(__segname("GAME_DATA"))
        small_font_glyphs[];

#define PLANAR_ROW_BYTES 80u
#define PLANAR_COUNT 4u
#define FONT_HEIGHT 5u

void far small_text_render(
        const u8 *text,
        u16 x,
        u16 y,
        u8 color);

#pragma aux small_text_render \
        parm [si] [ax] [bx] [dx] modify exact []

void far small_text_render(
        const u8 *text,
        u16 x,
        u16 y,
        u8 color)
{
    volatile u8 far *glyph_origin;
    volatile u8 far *row_origin;
    u16 glyph_offset;
    u16 plane_number;
    u16 row;
    u16 row_offset;
    u8 character;
    u8 glyph_index;
    u8 map_mask;
    u8 plane;
    u8 row_bits;

    _asm push eax;
    _asm push ds;
    _asm push es;

    row_offset = (u16)(y << 4);
    row_offset += (u16)(y << 6);
    plane = (u8)x & 3u;
    glyph_origin = graphics_draw_framebuffer + row_offset + (x >> 2);

    outportb(0x03c4u, 2u);
    map_mask = (u8)(0x11u << plane);
    for (;;) {
        outportb(0x03c5u, map_mask);
        character = *text++;
        if (character == 0) {
            break;
        }

        glyph_index = small_font_character_map[character];
        if ((i8)glyph_index >= 0) {
            glyph_offset = (u16)glyph_index * FONT_HEIGHT;
            for (plane_number = 0; plane_number < PLANAR_COUNT; ++plane_number) {
                row_origin = glyph_origin;
                if ((u16)plane + plane_number >= PLANAR_COUNT) {
                    ++row_origin;
                }
                for (row = 0; row < FONT_HEIGHT; ++row) {
                    row_bits = small_font_glyphs[glyph_offset + row];
                    if ((row_bits & (u8)(0x80u >> plane_number)) != 0) {
                        row_origin[0] = color;
                    }
                    row_origin += PLANAR_ROW_BYTES;
                }

                map_mask = (u8)((map_mask << 1) | (map_mask >> 7));
                outportb(0x03c5u, map_mask);
            }
        }

        ++glyph_origin;
    }

    _asm pop es;
    _asm pop ds;
    _asm pop eax;
}
