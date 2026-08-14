#include "../include/bloodprg_input.h"

void CB_NEAR mouse_hit_test(
    const volatile bloodprg_rect_i16 CB_NEAR *rect,
    volatile cb_u8 CB_NEAR *flags
)
{
    if ((mouse_primary_pressed & BLOODPRG_MOUSE_BUTTON_PRIMARY) == 0) {
        return;
    }
    if (mouse_x < rect->x || mouse_x - rect->width > rect->x) {
        return;
    }
    if (mouse_y < rect->y || mouse_y - rect->height > rect->y) {
        return;
    }
    *flags |= BLOODPRG_UI_HIT_FLAG;
}
