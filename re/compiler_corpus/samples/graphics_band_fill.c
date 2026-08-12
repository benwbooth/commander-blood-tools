/*
 * Codegen probe for BLOODPRG 0x003D7B/0x003DBF.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef volatile u8 FAR *buffer_pointer;

extern buffer_pointer GAME_DATA display_buffer;
extern volatile u16 GAME_DATA band_top_row;
extern volatile u16 GAME_DATA band_bottom_row;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define DISPLAY_BAND_AT(offset) \
    ((volatile u32 FAR *)MK_FP(FP_SEG(display_buffer), (offset)))
#else
#define DISPLAY_BAND_AT(offset) \
    ((volatile u32 FAR *)(display_buffer + (offset)))
#endif

void FAR graphics_band_fill_probe(u8 color);

#if defined(__WATCOMC__)
#pragma aux graphics_band_fill_probe parm [ax] modify exact []
#endif

void FAR graphics_band_fill_probe(u8 color)
{
    volatile u32 FAR *dst;
    u32 pattern;
    u16 top;
    u16 row_offset;
    u16 height;
    u16 count;
    u16 i;

    top = band_top_row;
    row_offset = (u16)((((top & 0x00ffu) << 8) | (top >> 8))
            + (u16)(top << 6));
    height = (u16)(band_bottom_row - top);
    count = (u16)(height * 80u);
    dst = DISPLAY_BAND_AT(row_offset);

    pattern = color;
    pattern |= pattern << 8;
    pattern |= pattern << 16;

    for (i = 0; i < count; ++i) {
        dst[i] = pattern;
    }
}
