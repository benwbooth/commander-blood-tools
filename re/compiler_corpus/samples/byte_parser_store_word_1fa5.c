/*
 * Codegen probe for BLOODPRG 0x0076BA.
 * This is not recovered game source.
 */
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

extern volatile u16 GAME_DATA parser_word;

#if defined(__WATCOMC__)
#pragma aux byte_parser_store_word_1fa5_probe parm [ds si] value [ds si] modify exact [ax si es]
#endif

const u16 FAR *NEAR byte_parser_store_word_1fa5_probe(
    const u16 FAR *script_words)
{
    parser_word = *script_words++;
    return script_words;
}
