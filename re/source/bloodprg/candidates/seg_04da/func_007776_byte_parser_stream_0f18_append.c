#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR byte_parser_stream_0f18_append(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_game_char_ptr destination;
    cb_u16 leading_word;
    cb_u8 ch;

    destination = byte_parser_stream_0f18_cursor;
    leading_word = *(const cb_u16 CB_FAR *)script_bytes;
    script_bytes += 2;
    *destination++ = (char)(cb_u8)leading_word;
    *destination++ = (char)(cb_u8)(leading_word >> 8);

    do {
        ch = *script_bytes++;
        *destination++ = (char)ch;
    } while (ch != '\0');

    byte_parser_stream_0f18_cursor = destination;
    return script_bytes;
}
