#include "../include/bloodprg_common.h"

void CB_NEAR mem_copy_words(cb_u16 *dst, const cb_u16 *src)
{
    cb_u16 word_count = 4u;

    do {
        *dst++ = *src++;
    } while (--word_count != 0u);
}
