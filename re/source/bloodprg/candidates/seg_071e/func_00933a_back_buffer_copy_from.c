#include "../include/bloodprg_graphics.h"

void CB_NEAR back_buffer_copy_from(cb_u16 x, cb_u16 y, cb_u16 width)
{
    bloodprg_graphics_buffer_ptr source;
    bloodprg_graphics_buffer_ptr destination;
    cb_u16 offset;

    /* Every recovered caller uses rows 0..199 and normalized buffer pointers. */
    offset = (cb_u16)(y * 320u + x);
    source = graphics_work_surface + offset;
    destination = graphics_back_buffer + offset;
    _fmemcpy((void CB_FAR *)destination,
            (const void CB_FAR *)source,
            width);

}
