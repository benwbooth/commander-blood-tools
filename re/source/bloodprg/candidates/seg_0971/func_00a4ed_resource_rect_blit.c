#include "../include/bloodprg_list.h"

#define RESOURCE_RECT_FRAMEBUFFER_WIDTH 320u
#define RESOURCE_RECT_TRANSPARENT_MODE 0xFFu

void CB_NEAR resource_rect_blit(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *framebuffer,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 row_mode)
{
    volatile cb_u8 CB_FAR *destination;
    cb_u16 count;
    cb_u16 pitch;
    cb_u8 rows;
    cb_u8 transparent;
    cb_u8 value;

    destination = framebuffer + (cb_u16)(
            ((y & 0x00FFu) << 8) +
            (y >> 8) +
            (y << 6) +
            x);
    rows = (cb_u8)row_mode;
    transparent = (cb_u8)(row_mode >> 8) ==
            RESOURCE_RECT_TRANSPARENT_MODE;

    if (width == RESOURCE_RECT_FRAMEBUFFER_WIDTH) {
        count = (cb_u16)(width * rows);
        if (transparent != 0u) {
            do {
                value = *source++;
                if (value != 0u) {
                    *destination = value;
                }
                ++destination;
            } while (--count != 0u);
        } else {
            while (count != 0u) {
                *destination++ = *source++;
                --count;
            }
        }
        return;
    }

    pitch = (cb_u16)(RESOURCE_RECT_FRAMEBUFFER_WIDTH - width);
    do {
        count = width;
        if (transparent != 0u) {
            do {
                value = *source++;
                if (value != 0u) {
                    *destination = value;
                }
                ++destination;
            } while (--count != 0u);
        } else {
            while (count != 0u) {
                *destination++ = *source++;
                --count;
            }
        }
        destination += pitch;
    } while (--rows != 0u);
}
