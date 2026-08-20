#include "../include/bloodprg_graphics.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define BLOODPRG_BACK_BAND_AT(offset) \
    ((volatile cb_u32 CB_FAR *)MK_FP(FP_SEG(graphics_back_buffer), (offset)))
#else
#define BLOODPRG_BACK_BAND_AT(offset) \
    ((volatile cb_u32 CB_FAR *)(graphics_back_buffer + (offset)))
#endif

void CB_SAVE_REGS CB_FAR back_buffer_fill(cb_u8 color)
{
    volatile cb_u32 CB_FAR *dst;
    cb_u32 pattern;
    cb_u16 top;
    cb_u16 row_offset;
    cb_u16 height;
    cb_u16 count;
    cb_u16 i;

#if defined(__WATCOMC__)
    _asm push eax;
#endif

    top = graphics_band_top_row;
    row_offset = (cb_u16)((((top & 0x00ffu) << 8) | (top >> 8))
            + (cb_u16)(top << 6));
    height = (cb_u16)(graphics_band_bottom_row - top);
    count = (cb_u16)(height * 80u);
    dst = BLOODPRG_BACK_BAND_AT(row_offset);

    pattern = color;
    pattern |= pattern << 8;
    pattern |= pattern << 16;

    for (i = 0; i < count; ++i) {
        dst[i] = pattern;
    }

#if defined(__WATCOMC__)
    _asm pop eax;
#endif
}
