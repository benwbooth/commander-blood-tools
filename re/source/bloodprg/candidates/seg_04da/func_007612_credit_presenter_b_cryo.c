#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR credit_presenter_b_cryo(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 dst_index;
    cb_u8 ch;

    dst_index = 0u;
    do {
        ch = *script_bytes++;
        credit_text_buffer[dst_index++] = (char)ch;
    } while (ch != '\0');

    credit_reveal_active = 1;
    credit_reveal_timer = 0;
    return script_bytes;
}
