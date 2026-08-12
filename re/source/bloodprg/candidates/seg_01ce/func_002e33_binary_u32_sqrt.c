#include "../include/bloodprg_ship3d.h"

cb_u16 CB_FAR binary_u32_sqrt(cb_u32 value)
{
    cb_u16 low;
    cb_u16 high;
    cb_u16 estimate;

    low = (cb_u16)value;
    high = (cb_u16)(value >> 16);

    if (high != 0) {
        estimate = 0x0fffu;
        if ((high & 0xff00u) != 0) {
            estimate = 0xffffu;
            if (high >= 0xfffeu) {
                return low;
            }
        }
    } else {
        if (low == 0) {
            return low;
        }
        estimate = 0x000fu;
        if ((low & 0xff00u) != 0) {
            estimate = 0x00ffu;
        }
    }

    for (;;) {
        cb_u16 quotient;
        cb_u16 candidate;

        quotient = (cb_u16)(value / estimate);
        candidate = (cb_u16)(((cb_u32)quotient + estimate) >> 1);
        if (candidate >= estimate) {
            return candidate;
        }
        estimate = candidate;
    }
}
