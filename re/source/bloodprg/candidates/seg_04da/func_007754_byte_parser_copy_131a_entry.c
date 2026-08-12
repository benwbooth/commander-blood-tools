#include "../include/bloodprg_byte_parser.h"

void CB_NEAR byte_parser_copy_131a_entry(const cb_u8 **script_bytes)
{
    volatile char *dst;
    cb_u8 ch;

    dst = byte_parser_table_131a_cursor;
    for (;;) {
        ch = **script_bytes;
        if ((ch & 0x80u) != 0 || ch < 0x20u) {
            break;
        }
        *dst = (char)ch;
        ++dst;
        ++*script_bytes;
    }
    *dst = '\0';
    byte_parser_table_131a_cursor += 0x10;
    ++byte_parser_table_131e_count;
}
