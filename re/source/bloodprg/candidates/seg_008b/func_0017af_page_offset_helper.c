#include <conio.h>

#include "../include/bloodprg_graphics.h"

void CB_NEAR page_offset_helper(void)
{
    cb_i16 offset;

    offset = graphics_draw_page_offset;
    if (offset < 0) {
        offset = 0;
    } else {
        offset = (cb_i16)((cb_u16)offset + 0x4000u);
    }
    graphics_draw_page_offset = offset;

    offset = graphics_screen_page_offset;
    if (offset < 0) {
        offset = 0;
    } else {
        offset = (cb_i16)((cb_u16)offset + 0x4000u);
    }
    graphics_screen_page_offset = offset;

    outpw(video_crtc_base_port_ds, ((cb_u16)offset & 0xff00u) | 0x000cu);
}
