#include "../include/bloodprg_ship3d.h"

void CB_NEAR ship_3d_plot_point(volatile ship_3d_projection_context *projection,
        volatile cb_u8 CB_FAR *framebuffer)
{
    cb_i16 x;
    cb_i16 y;
    cb_u16 offset;
    cb_u8 shade;

    x = (cb_i16)projection->projected_x;
    if (x < ship_3d_clip_left || x >= ship_3d_clip_right) {
        return;
    }

    y = (cb_i16)projection->projected_y;
    if (y < ship_3d_clip_top || y >= ship_3d_clip_bottom) {
        return;
    }

    offset = (cb_u16)((cb_u16)y * 320u + (cb_u16)x);
    if (framebuffer[offset] != 0) {
        return;
    }

    shade = (cb_u8)(0xefu - (projection->projected_depth >> 12));
    framebuffer[offset] = shade;
}
