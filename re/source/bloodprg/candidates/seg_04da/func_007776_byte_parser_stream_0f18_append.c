#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR byte_parser_stream_0f18_append(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 dst_offset;
    cb_u16 leading_word;
    cb_u8 ch;

    dst_offset = (cb_u16)byte_parser_stream_0f18_cursor;
    leading_word = *(const cb_u16 CB_FAR *)script_bytes;
    script_bytes += 2;
    byte_parser_stream_segment[dst_offset++] = (cb_u8)leading_word;
    byte_parser_stream_segment[dst_offset++] = (cb_u8)(leading_word >> 8);

    do {
        ch = *script_bytes++;
        byte_parser_stream_segment[dst_offset++] = ch;
    } while (ch != '\0');

    byte_parser_stream_0f18_cursor = (cb_game_char_ptr)dst_offset;
    return script_bytes;
}
