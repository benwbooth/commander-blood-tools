#include "../include/bloodprg_common.h"

#define BLOODPRG_DECIMAL_SCRATCH_END 11u

void CB_FAR decimal_append_i32(cb_i32 value, char CB_FAR *destination)
{
    cb_u8 CB_CODE_DATA *cursor;
    cb_u32 magnitude;
    cb_u32 quotient;

    cursor = decimal_append_scratch + BLOODPRG_DECIMAL_SCRATCH_END;
    if (value < 0) {
        *destination++ = '-';
        magnitude = 0UL - (cb_u32)value;
    } else {
        magnitude = (cb_u32)value;
    }

    do {
        quotient = magnitude / 10UL;
        *--cursor = (cb_u8)('0' + (cb_u8)(magnitude - quotient * 10UL));
        magnitude = quotient;
    } while (magnitude != 0);

    do {
        *destination++ = (char)*cursor;
    } while (*cursor++ != 0);
}
