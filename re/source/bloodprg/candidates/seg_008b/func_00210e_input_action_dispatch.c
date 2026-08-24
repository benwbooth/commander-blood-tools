#include "../include/bloodprg_input.h"
#include "../include/bloodprg_platform.h"

static const cb_i8 CB_CODE_DATA input_action_translation[256] = {
    -1, -1, -1, -1, -1, -1, -1, -1,  8, -1, -1, -1, -1,  6, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,  7, -1, -1, -1, -1,
     7,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,
     8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,
     8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,
    15,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,
     8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,
    15,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,  8,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,  5,  9, 10, 11, 12,
    13, 14, -1, -1, -1, -1, -1, -1,  0, -1, -1,  3, -1,  2, -1, -1,
     1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1
};

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
    switch (action_index) {
    case 0:
        input_action_move_previous(raw_low_byte);
        break;
    case 1:
        input_action_move_next(raw_low_byte);
        break;
    case 2:
        input_action_noop_2(raw_low_byte);
        break;
    case 3:
        input_action_noop_3(raw_low_byte);
        break;
    case 4:
        input_action_request_shutdown(raw_low_byte);
        break;
    case 5:
        input_action_noop_5(raw_low_byte);
        break;
    case 6:
        input_action_accept(raw_low_byte);
        break;
    case 7:
        input_action_cancel(raw_low_byte);
        break;
    case 8:
        input_action_latch_text_key(raw_low_byte);
        break;
    case 9:
        input_action_noop_9(raw_low_byte);
        break;
    case 10:
        input_action_noop_10(raw_low_byte);
        break;
    case 11:
        input_action_noop_11(raw_low_byte);
        break;
    case 12:
        input_action_noop_12(raw_low_byte);
        break;
    case 13:
        input_action_noop_13(raw_low_byte);
        break;
    case 14:
        input_action_noop_14(raw_low_byte);
        break;
    case 15:
        input_action_toggle_pause(raw_low_byte);
        break;
    default:
        break;
    }
}
