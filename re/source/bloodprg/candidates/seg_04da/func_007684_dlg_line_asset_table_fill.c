#include "../include/bloodprg_byte_parser.h"

void CB_NEAR dlg_line_asset_table_fill(const cb_u8 **script_bytes)
{
    cb_u8 id;
    cb_u16 stored_id;
    volatile cb_u16 *asset_cursor;
    volatile char *detail_cursor;
    cb_u8 ch;

    id = **script_bytes;
    ++*script_bytes;
    if ((id & 0x80u) != 0) {
        stored_id = (cb_u16)(int)(cb_i8)id;
    } else {
        stored_id = (cb_u16)(0x0dd7u + (((cb_u16)id - 1u) << 4));
    }

    asset_cursor = byte_parser_asset_cursor;
    *asset_cursor = stored_id;
    byte_parser_asset_cursor = asset_cursor + 2;

    detail_cursor = byte_parser_detail_cursor;
    byte_parser_detail_cursor = detail_cursor + 0x1a;
    for (;;) {
        ch = **script_bytes;
        if ((ch & 0x80u) != 0 || ch < 0x20u) {
            break;
        }
        *detail_cursor = (char)ch;
        ++detail_cursor;
        ++*script_bytes;
    }
    *detail_cursor = '\0';
}
