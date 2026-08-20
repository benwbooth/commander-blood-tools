#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_save.h"

void CB_NEAR input_action_toggle_pause(cb_u8 raw_low_byte)
{
    if ((save_request_active & 1u) == 0u) {
        main_loop_hud_refresh_enabled =
                (main_loop_hud_refresh_enabled & 1u) == 0u;
    }
    input_action_latch_text_key(raw_low_byte);
}
