#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_NEAR *CB_NEAR byte_parser_stream_0f18_append(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_game_char_ptr dst;
    cb_u8 ch;

    dst = byte_parser_stream_0f18_cursor;
    *(cb_game_word_ptr)dst = *(const cb_u16 CB_NEAR *)script_bytes;
    dst += 2;
    script_bytes += 2;

    do {
        ch = *script_bytes++;
        *dst++ = (char)ch;
    } while (ch != '\0');

    byte_parser_stream_0f18_cursor = dst;
    return script_bytes;
}
