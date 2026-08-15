#include "../include/bloodprg_ship3d.h"

typedef union ship_3d_depth_word {
    cb_u16 value;
    struct {
        cb_u8 low;
        cb_u8 high;
    } byte;
} ship_3d_depth_word;

void CB_NEAR ship_3d_depth_scroll_step(void)
{
    ship_3d_depth_word depth;

    if ((ship_3d_depth_opening & 1u) != 0) {
        depth.value = ship_3d_depth_offset_ds;
        if (depth.value == 0x0041u) {
            ship_3d_depth_opening = 0;
            return;
        }

        depth.byte.low += ship_3d_depth_step;
        if ((cb_i16)depth.value < (cb_i16)0x0041) {
            ship_3d_depth_offset_ds = depth.value;
        } else {
            ship_3d_depth_offset_ds = 0x0041u;
        }
        return;
    }

    if ((ship_3d_depth_closing & 1u) == 0) {
        return;
    }

    depth.value = ship_3d_depth_offset_ds;
    if (depth.value == 0) {
        ship_3d_depth_closing = 0;
        return;
    }

    depth.byte.low -= ship_3d_depth_step;
    if ((cb_i8)depth.byte.low >= 0) {
        ship_3d_depth_offset_ds = depth.value;
    } else {
        ship_3d_depth_offset_ds = 0;
    }
}
