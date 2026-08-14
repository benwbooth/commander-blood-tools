#include "../include/bloodprg_nav.h"

#define PRESENTATION_MODE_GATE 0x50u
#define PRESENTATION_MODE_SECOND_RECT 0x40u
#define PRESENTATION_MODE_ACTIVE_FLAG 0x01u

void CB_NEAR presentation_mode_dispatch(void)
{
    const volatile bloodprg_rect_i16 CB_NEAR *rect;
    cb_i16 point_x;
    cb_i16 point_y;

    if ((vm_ui_flags & PRESENTATION_MODE_GATE) == 0u) {
        return;
    }

    rect = &nav_actor_slots[0].hit_rect;
    if ((vm_ui_flags & PRESENTATION_MODE_SECOND_RECT) != 0u) {
        rect = (const volatile bloodprg_rect_i16 CB_NEAR *)
            ((const volatile cb_u8 CB_NEAR *)rect
             + 2u * sizeof(bloodprg_nav_actor_slot));
    }

    point_x = mouse_x;
    if (point_x < rect->x) {
        goto outside;
    }
    point_x = (cb_i16)((cb_u16)point_x - (cb_u16)rect->width);
    if (point_x > rect->x) {
        goto outside;
    }

    point_y = mouse_y;
    if (point_y < rect->y) {
        goto outside;
    }
    point_y = (cb_i16)((cb_u16)point_y - (cb_u16)rect->height);
    if (point_y > rect->y) {
        goto outside;
    }

    if ((presentation_mode_active & PRESENTATION_MODE_ACTIVE_FLAG) == 0u) {
        presentation_mode_active = PRESENTATION_MODE_ACTIVE_FLAG;
        nav_actor_presentation_state = 9u;
    }
    return;

outside:
    if ((presentation_mode_active & PRESENTATION_MODE_ACTIVE_FLAG) != 0u) {
        presentation_mode_active = 0u;
        nav_actor_presentation_state = presentation_mode_previous_state;
    }
}
