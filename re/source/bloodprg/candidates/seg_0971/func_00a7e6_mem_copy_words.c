#include "../include/bloodprg_common.h"

void CB_NEAR mem_copy_words(cb_u16 *dst, const cb_u16 *src)
{
    dst[0] = src[0];
    dst[1] = src[1];
    dst[2] = src[2];
    dst[3] = src[3];
}
