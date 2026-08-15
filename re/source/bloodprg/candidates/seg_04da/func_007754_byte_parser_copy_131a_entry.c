#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_131a_entry(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 slot_offset;
    cb_u16 dst_offset;
    cb_u8 ch;

    slot_offset = (cb_u16)byte_parser_table_131a_cursor;
    dst_offset = slot_offset;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        byte_parser_stream_segment[dst_offset++] = ch;
    }
    byte_parser_stream_segment[dst_offset] = 0u;
    byte_parser_table_131a_cursor =
            (cb_game_char_ptr)(cb_u16)(slot_offset + 0x10u);
    ++byte_parser_table_131e_count;
    return script_bytes;
}
