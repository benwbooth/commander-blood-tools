/*
 * Codegen probe for BLOODPRG 0x00647B.
 * This is not recovered game source.
 */
typedef signed int i16;
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

extern volatile u16 GAME_DATA operand_word_count;

#if defined(__WATCOMC__)
#pragma aux scan_zero_word_probe parm [si] modify exact [ax]
#endif

void NEAR scan_zero_word_probe(const i16 NEAR *script_words)
{
    u16 count;

    count = 0;
    while (count != 0xffffu && *script_words > 0) {
        ++script_words;
        ++count;
    }

    operand_word_count = count;
}
