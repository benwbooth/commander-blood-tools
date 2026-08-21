#include "../include/bloodprg_common.h"

#define BLOODPRG_DECIMAL_SCRATCH_END 11u

cb_u8 decimal_append_scratch[12] = {0};

void CB_FAR decimal_append_i16(cb_i16 value, char CB_FAR *destination)
{
    cb_u8 *cursor;
    cb_u16 magnitude;
    cb_u16 quotient;

    cursor = decimal_append_scratch + BLOODPRG_DECIMAL_SCRATCH_END;
    if (value < 0) {
        *destination++ = '-';
        magnitude = (cb_u16)(0u - (cb_u16)value);
    } else {
        magnitude = (cb_u16)value;
    }

    do {
        quotient = magnitude / 10u;
        *--cursor = (cb_u8)('0' + magnitude - quotient * 10u);
        magnitude = quotient;
    } while (magnitude != 0);

    do {
        *destination++ = (char)*cursor;
    } while (*cursor++ != 0);
}
