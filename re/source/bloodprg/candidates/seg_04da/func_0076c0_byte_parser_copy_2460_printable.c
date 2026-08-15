#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_2460_printable(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 dst_index;
    cb_u8 ch;

    dst_index = 0u;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        byte_parser_table_2460[dst_index++] = (char)ch;
    }
    byte_parser_table_2460[dst_index] = '\0';
    return script_bytes;
}
