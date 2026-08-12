#include "../include/bloodprg_graphics.h"

void CB_FAR blit_fill_row_5221(cb_u8 color)
{
    volatile cb_u32 CB_FAR *dst;
    cb_u32 pattern;
    cb_u16 top;
    cb_u16 height;
    cb_u16 i;

    pattern = color;
    pattern |= pattern << 8;
    pattern |= pattern << 16;

    top = graphics_band_top_row;
    height = (cb_u16)(graphics_band_bottom_row - top);
    dst = (volatile cb_u32 CB_FAR *)(graphics_display_buffer + top * 320u);

    for (i = 0; i < (cb_u16)(height * 80u); ++i) {
        dst[i] = pattern;
    }
}
