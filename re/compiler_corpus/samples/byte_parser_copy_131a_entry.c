/*
 * Codegen probe for BLOODPRG 0x007754.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed char i8;

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

extern volatile game_char_ptr GAME_DATA table_cursor;
extern volatile u8 GAME_DATA table_count;

#if defined(__WATCOMC__)
#pragma aux byte_parser_copy_131a_entry_probe parm [ds si] value [ds si] modify exact [ax si di es]
#endif

const u8 FAR *NEAR byte_parser_copy_131a_entry_probe(
    const u8 FAR *script_bytes)
{
    game_char_ptr dst;
    u8 ch;

    dst = table_cursor;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';
    table_cursor += 0x10;
    ++table_count;
    return script_bytes;
}
