#include "../include/bloodprg_input.h"

void CB_NEAR mouse_hit_test(
    const volatile bloodprg_rect_i16 CB_NEAR *rect,
    volatile cb_u8 CB_NEAR *flags
)
{
    cb_i16 coordinate;

    if ((mouse_primary_pressed & BLOODPRG_MOUSE_BUTTON_PRIMARY) == 0) {
        return;
    }

    coordinate = mouse_x;
    if (coordinate < rect->x) {
        return;
    }
    coordinate = (cb_i16)((cb_u16)coordinate - (cb_u16)rect->width);
    if (coordinate > rect->x) {
        return;
    }

    coordinate = mouse_y;
    if (coordinate < rect->y) {
        return;
    }
    coordinate = (cb_i16)((cb_u16)coordinate - (cb_u16)rect->height);
    if (coordinate > rect->y) {
        return;
    }

    *flags |= BLOODPRG_UI_HIT_FLAG;
}
