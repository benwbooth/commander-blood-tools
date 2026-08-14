#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR credit_presenter_b_cryo(
    const cb_u8 CB_FAR *script_bytes)
{
    char CB_GAME_DATA *dst;
    cb_u8 ch;

    dst = credit_text_buffer;
    do {
        ch = *script_bytes++;
        *dst++ = (char)ch;
    } while (ch != '\0');

    credit_reveal_active = 1;
    credit_reveal_timer = 0;
    return script_bytes;
}
