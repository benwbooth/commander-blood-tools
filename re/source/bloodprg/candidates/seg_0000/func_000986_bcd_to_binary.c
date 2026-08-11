#include "../include/bloodprg_common.h"

cb_u8 CB_NEAR bcd_to_binary(cb_u8 value)
{
    cb_u8 low;
    cb_u8 high;

    low = (cb_u8)(value & 0x0fu);
    high = (cb_u8)(value >> 4);

    return (cb_u8)((high * 10u) + low);
}
