/*
 * Codegen probe for BLOODPRG 0x0077A9.
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

extern char GAME_DATA music_name_field[];
extern volatile u8 GAME_DATA music_name_unchanged;
extern volatile u8 GAME_DATA music_name_changed;

#if defined(__WATCOMC__)
#pragma aux music_voc_name_patcher_probe parm [si] value [si] modify exact [ax si di]
#endif

const u8 NEAR *NEAR music_voc_name_patcher_probe(
    const u8 NEAR *script_bytes)
{
    char GAME_DATA *dst;
    u8 ch;

    dst = music_name_field;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch <= 0x20u) {
            --script_bytes;
            break;
        }
        if (ch >= 0x61u) {
            ch = (u8)(ch & 0xdfu);
        }
        if (ch != (u8)*dst) {
            music_name_changed = 1;
        }
        *dst++ = (char)ch;
    }

    if ((music_name_changed & 1u) == 0) {
        music_name_unchanged |= 1;
    }
    *dst = '\0';
    return script_bytes;
}
