#include "../include/bloodprg_graphics.h"

void CB_FAR full_screen_blit(const cb_u32 *source)
{
    volatile cb_u32 CB_FAR *dst;
    cb_u16 i;

    dst = (volatile cb_u32 CB_FAR *)graphics_display_buffer;
    for (i = 0; i < 0x3e80u; ++i) {
        dst[i] = source[i];
    }
}
