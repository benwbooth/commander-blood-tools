/*
 * Codegen probe for BLOODPRG 0x00763E.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed char i8;
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

extern char GAME_DATA snd_bank_name_field[];
extern volatile char GAME_DATA snd_bank_path[];
extern volatile u16 GAME_DATA ui_state;
void FAR snd_bank_loader_probe(u16 mode, const volatile char NEAR *path);

#if defined(__WATCOMC__)
#pragma aux snd_bank_loader_probe parm [ax] [si] modify exact []
#pragma aux byte_parser_snd_bank_name_load_probe parm [ds si] value [ds si] modify exact [ax bx cx dx si di es]
#endif

const u8 FAR *NEAR byte_parser_snd_bank_name_load_probe(
    const u8 FAR *script_bytes)
{
    char GAME_DATA *dst;
    u8 ch;

    dst = snd_bank_name_field;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';

    if ((ui_state & 1u) == 0) {
        snd_bank_loader_probe(1u, snd_bank_path);
    }
    return script_bytes;
}
