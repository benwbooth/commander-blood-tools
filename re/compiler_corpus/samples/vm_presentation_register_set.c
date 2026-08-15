/*
 * Codegen probe for BLOODPRG 0x0067BA.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__WATCOMC__)
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define NEAR near
#define GAME_DATA far
#else
#define NEAR
#define GAME_DATA
#endif

extern volatile u8 GAME_DATA presentation_active;
extern volatile u16 GAME_DATA presentation_register;

#if defined(__WATCOMC__)
#pragma aux vm_presentation_register_set_probe parm [si] value [si] modify exact [ax si]
#endif

const u16 NEAR *NEAR vm_presentation_register_set_probe(
        const u16 NEAR *script_words)
{
    u16 value;

    value = *script_words++;
    if ((presentation_active & 1u) != 0) {
        presentation_register = value;
    }

    return script_words;
}
