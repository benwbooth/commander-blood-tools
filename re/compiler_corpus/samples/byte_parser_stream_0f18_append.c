/*
 * Codegen probe for BLOODPRG 0x007776.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef volatile char GAME_DATA *game_char_ptr;
typedef volatile u16 GAME_DATA *game_word_ptr;

extern volatile game_char_ptr GAME_DATA stream_cursor;

#if defined(__WATCOMC__)
#pragma aux byte_parser_stream_0f18_append_probe parm [ds si] value [ds si] modify exact [ax si di es]
#endif

const u8 FAR *NEAR byte_parser_stream_0f18_append_probe(
    const u8 FAR *script_bytes)
{
    game_char_ptr dst;
    u8 ch;

    dst = stream_cursor;
    *(game_word_ptr)dst = *(const u16 FAR *)script_bytes;
    dst += 2;
    script_bytes += 2;

    do {
        ch = *script_bytes++;
        *dst++ = (char)ch;
    } while (ch != '\0');

    stream_cursor = dst;
    return script_bytes;
}
