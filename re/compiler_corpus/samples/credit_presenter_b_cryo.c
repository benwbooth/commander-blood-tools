/*
 * Codegen probe for BLOODPRG 0x007612.
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

extern char GAME_DATA credit_text_buffer[];
extern volatile u8 GAME_DATA credit_reveal_active;
extern volatile u16 GAME_DATA credit_reveal_timer;

#if defined(__WATCOMC__)
#pragma aux credit_presenter_b_cryo_probe parm [ds si] value [ds si] modify exact [ax si di es]
#endif

const u8 FAR *NEAR credit_presenter_b_cryo_probe(
    const u8 FAR *script_bytes)
{
    char GAME_DATA *dst;
    u8 ch;

    dst = credit_text_buffer;
    do {
        ch = *script_bytes++;
        *dst++ = (char)ch;
    } while (ch != '\0');

    credit_reveal_active = 1;
    credit_reveal_timer = 0;
    return script_bytes;
}
