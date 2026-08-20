#include "../include/bloodprg_input.h"

void CB_NEAR input_action_latch_text_key(cb_u8 raw_low_byte)
{
    input_dispatch_state_b15 = raw_low_byte;
}
