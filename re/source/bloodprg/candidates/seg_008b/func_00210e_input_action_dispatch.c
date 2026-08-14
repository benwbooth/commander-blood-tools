#include "../include/bloodprg_input.h"
#include "../include/bloodprg_platform.h"

void CB_FAR input_action_dispatch(void)
{
    cb_u16 key;
    cb_u8 raw_low_byte;
    cb_u8 translated_code;
    cb_i8 action_index;

    input_dispatch_state_b15 = 0u;
    key = kbd_read_int16();
    if (key == 0u) {
        return;
    }

    raw_low_byte = (cb_u8)key;
    translated_code = raw_low_byte;
    if (translated_code == 0u) {
        translated_code = (cb_u8)((key >> 8) | 0x80u);
    }

    action_index = input_action_translation[translated_code];
    if (action_index >= 0) {
        input_action_handlers[(cb_u8)action_index](raw_low_byte);
    }
}
