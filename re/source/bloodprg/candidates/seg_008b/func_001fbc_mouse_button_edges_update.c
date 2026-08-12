#include "../include/bloodprg_input.h"

cb_u16 CB_NEAR mouse_button_edges_update(void)
{
    cb_u8 buttons;
    cb_u16 current_word;

    buttons = (cb_u8)mouse_button_state;
    if ((buttons & BLOODPRG_MOUSE_BUTTON_PRIMARY) != 0) {
        if ((buttons &= (cb_u8)mouse_previous_button_state) == 0) {
            mouse_primary_pressed = 1;
            mouse_press_pending = 1;
        }
    }

    if ((buttons & BLOODPRG_MOUSE_BUTTON_SECONDARY) != 0) {
        if ((buttons &= (cb_u8)mouse_previous_button_state) == 0) {
            mouse_secondary_pressed = 1;
            mouse_press_pending = 1;
        }
    }

    current_word = mouse_button_state;
    mouse_previous_button_state = current_word;
    return current_word;
}
