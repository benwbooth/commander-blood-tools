#include "../include/bloodprg_graphics.h"

void CB_FAR composite_draw_a(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height)
{
    gfx_horizontal_span(color, x, y, width);
    gfx_vertical_span(color, x, y, height);
    gfx_vertical_span(
            color, (cb_u16)(x + width - 1u), y, height);
    gfx_horizontal_span(
            color, x, (cb_u16)(y + height - 1u), width);
}
