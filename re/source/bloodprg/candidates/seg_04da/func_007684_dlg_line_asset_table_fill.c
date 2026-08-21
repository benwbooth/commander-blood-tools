#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR dlg_line_asset_table_fill(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_game_word_ptr asset;
    cb_game_char_ptr detail;
    cb_u16 stored_id;
    cb_u8 ch;

    stored_id = (cb_u16)(cb_i16)(cb_i8)*script_bytes++;
    /*
     * The opcode-0x07 dispatcher leaves SF clear. CBW does not change flags,
     * so the assembly's following JS is unreachable through that caller.
     */
    stored_id = (cb_u16)(0x0dd7u + ((stored_id - 1u) << 4));

    asset = byte_parser_asset_cursor;
    *asset = stored_id;
    byte_parser_asset_cursor = asset + 2u;

    detail = byte_parser_detail_cursor;
    byte_parser_detail_cursor = detail + 0x1au;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *detail++ = (char)ch;
    }
    *detail = '\0';
    return script_bytes;
}
