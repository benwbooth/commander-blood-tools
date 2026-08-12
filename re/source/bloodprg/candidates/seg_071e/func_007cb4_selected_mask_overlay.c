#include "../include/bloodprg_graphics.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define BLOODPRG_DISPLAY_AT(offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(graphics_display_buffer), (offset)))
#else
#define BLOODPRG_DISPLAY_AT(offset) (graphics_display_buffer + (offset))
#endif

#define BLOODPRG_MASK_COLOR 0xfeu
#define BLOODPRG_MASK_ROWS 16u
#define BLOODPRG_FRAMEBUFFER_WIDTH 320u
#define BLOODPRG_MASK_DESTINATION 0x12c5u

void CB_NEAR selected_mask_overlay(void)
{
    const cb_u8 CB_NEAR *source;
    volatile cb_u8 CB_FAR *row_pixels;
    unsigned row;

    source = selected_mask_rows[(cb_i16)selected_mask_index];
    row_pixels = BLOODPRG_DISPLAY_AT(BLOODPRG_MASK_DESTINATION);

    for (row = 0; row != BLOODPRG_MASK_ROWS; ++row) {
        volatile cb_u8 CB_FAR *pixel;
        cb_u16 bits;

        bits = (cb_u16)((cb_u16)source[0] << 8);
        bits = (cb_u16)(bits | source[1]);
        source += 2;
        pixel = row_pixels;

        while (bits != 0) {
            if ((bits & 0x8000u) != 0) {
                *pixel = BLOODPRG_MASK_COLOR;
            }
            ++pixel;
            bits = (cb_u16)(bits << 1);
        }

        row_pixels += BLOODPRG_FRAMEBUFFER_WIDTH;
    }
}
