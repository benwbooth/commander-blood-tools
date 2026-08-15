#include "../include/bloodprg_input.h"

int CB_FAR region_record_hittest(
        const volatile bloodprg_rect_i16 CB_NEAR *rect)
{
    cb_i16 coordinate;

    if ((mouse_primary_pressed & BLOODPRG_MOUSE_BUTTON_PRIMARY) == 0) {
        return 0;
    }

    coordinate = mouse_x;
    if (coordinate < rect->x) {
        return 0;
    }
    coordinate = (cb_i16)((cb_u16)coordinate - (cb_u16)rect->width);
    if (coordinate > rect->x) {
        return 0;
    }

    coordinate = mouse_y;
    if (coordinate < rect->y) {
        return 0;
    }
    coordinate = (cb_i16)((cb_u16)coordinate - (cb_u16)rect->height);
    if (coordinate > rect->y) {
        return 0;
    }

    return 1;
}
