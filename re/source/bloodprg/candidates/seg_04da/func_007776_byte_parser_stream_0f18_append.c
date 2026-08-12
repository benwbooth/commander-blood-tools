#include "../include/bloodprg_byte_parser.h"

void CB_NEAR byte_parser_stream_0f18_append(const cb_u8 **script_bytes)
{
    volatile char *dst;
    char ch;

    dst = byte_parser_stream_0f18_cursor;
    *(volatile cb_u16 *)dst = *(const cb_u16 *)*script_bytes;
    dst += 2;
    *script_bytes += 2;

    do {
        ch = (char)**script_bytes;
        ++*script_bytes;
        *dst = ch;
        ++dst;
    } while (ch != '\0');

    byte_parser_stream_0f18_cursor = (volatile char *)dst;
}
