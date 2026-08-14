#include "../include/bloodprg_input.h"

int CB_FAR region_record_hittest(
        const volatile bloodprg_rect_i16 CB_NEAR *rect)
{
    if ((mouse_primary_pressed & BLOODPRG_MOUSE_BUTTON_PRIMARY) == 0) {
        return 0;
    }
    if (mouse_x < rect->x || mouse_x - rect->width > rect->x) {
        return 0;
    }
    if (mouse_y < rect->y || mouse_y - rect->height > rect->y) {
        return 0;
    }
    return 1;
}
