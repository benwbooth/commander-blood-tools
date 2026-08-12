#include "../include/bloodprg_graphics.h"

bloodprg_layout_offset_result CB_FAR layout_offset_calc(cb_u16 columns,
        cb_u16 rows)
{
    cb_u16 width;
    cb_u16 height;
    cb_u16 x;
    cb_u16 y;
    bloodprg_layout_offset_result result;

    width = (cb_u16)(columns * 4u + 4u);
    height = (cb_u16)(rows * 6u + 4u);
    x = (cb_u16)((320u - width) >> 1);
    y = (cb_u16)((200u - height) >> 1);

    blit_coord_guard_c(0, x, y, width, height);
    composite_draw_a(0x0fu, x, y, width, height);

    result.x = (cb_u16)(x + 2u);
    result.y = (cb_u16)(y + 2u);
    return result;
}
