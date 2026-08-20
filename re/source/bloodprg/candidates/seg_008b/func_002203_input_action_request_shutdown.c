#include "../include/bloodprg_input.h"
#include "../include/bloodprg_ship3d.h"

void CB_NEAR input_action_request_shutdown(cb_u8 raw_low_byte)
{
    (void)raw_low_byte;
    ship_3d_nav_choice_sound_gate = 1u;
}
