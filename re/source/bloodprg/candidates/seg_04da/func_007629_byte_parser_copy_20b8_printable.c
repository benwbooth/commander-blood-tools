#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_NEAR *CB_NEAR byte_parser_copy_20b8_printable(
    const cb_u8 CB_NEAR *script_bytes)
{
    char CB_GAME_DATA *dst;
    cb_u8 ch;

    dst = byte_parser_text_20b8;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';
    return script_bytes;
}
