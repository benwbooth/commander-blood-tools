#include "../include/bloodprg_ship3d.h"

void CB_NEAR ship_3d_depth_scroll_step(void)
{
    cb_u16 depth;
    cb_u8 low;

    if ((ship_3d_depth_opening & 1u) != 0) {
        depth = ship_3d_depth_offset;
        if (depth == 0x0041u) {
            ship_3d_depth_opening = 0;
            return;
        }

        low = (cb_u8)((cb_u8)depth + ship_3d_depth_step);
        depth = (cb_u16)((depth & 0xff00u) | low);
        if ((cb_i16)depth < (cb_i16)0x0041) {
            ship_3d_depth_offset = depth;
        } else {
            ship_3d_depth_offset = 0x0041u;
        }
        return;
    }

    if ((ship_3d_depth_closing & 1u) == 0) {
        return;
    }

    depth = ship_3d_depth_offset;
    if (depth == 0) {
        ship_3d_depth_closing = 0;
        return;
    }

    low = (cb_u8)((cb_u8)depth - ship_3d_depth_step);
    if ((low & 0x80u) == 0) {
        ship_3d_depth_offset = (cb_u16)((depth & 0xff00u) | low);
    } else {
        ship_3d_depth_offset = 0;
    }
}
