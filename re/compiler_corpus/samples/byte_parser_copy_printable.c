/*
 * Codegen probe for BLOODPRG 0x007629/0x00766F/0x0076C0/0x0076D5.
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

extern char GAME_DATA parser_text[];

#if defined(__WATCOMC__)
#pragma aux byte_parser_copy_printable_probe parm [si] value [si] modify exact [ax si di]
#endif

const u8 NEAR *NEAR byte_parser_copy_printable_probe(
    const u8 NEAR *script_bytes)
{
    char GAME_DATA *dst;
    u8 ch;

    dst = parser_text;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';
    return script_bytes;
}
