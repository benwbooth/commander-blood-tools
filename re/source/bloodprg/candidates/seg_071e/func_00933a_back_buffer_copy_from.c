#include "../include/bloodprg_graphics.h"

void CB_NEAR back_buffer_copy_from(cb_u16 x, cb_u16 y, cb_u16 width)
{
    cb_u16 offset;
    cb_u16 i;

    offset = (cb_u16)(y * 320u + x);
    for (i = 0; i < width; ++i) {
        graphics_back_buffer[offset + i] = graphics_work_surface[offset + i];
    }
}
