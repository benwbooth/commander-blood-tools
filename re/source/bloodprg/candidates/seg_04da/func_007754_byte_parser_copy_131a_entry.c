#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_131a_entry(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_game_char_ptr dst;
    cb_u8 ch;

    dst = byte_parser_table_131a_cursor;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';
    byte_parser_table_131a_cursor += 0x10;
    ++byte_parser_table_131e_count;
    return script_bytes;
}
