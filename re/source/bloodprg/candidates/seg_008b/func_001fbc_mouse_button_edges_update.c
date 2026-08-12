#include "../include/bloodprg_input.h"

void CB_NEAR mouse_button_edges_update(void)
{
    cb_u8 current;
    cb_u8 shared;

    current = (cb_u8)mouse_button_state;
    shared = (cb_u8)(current & (cb_u8)mouse_previous_button_state);

    if ((current & BLOODPRG_MOUSE_BUTTON_PRIMARY) != 0) {
        if (shared == 0) {
            mouse_primary_pressed = 1;
            mouse_press_pending = 1;
        }
    } else if ((current & BLOODPRG_MOUSE_BUTTON_SECONDARY) != 0) {
        if (shared == 0) {
            mouse_secondary_pressed = 1;
            mouse_press_pending = 1;
        }
    }

    mouse_previous_button_state = mouse_button_state;
}
