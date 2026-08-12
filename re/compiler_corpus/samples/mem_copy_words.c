/*
 * Codegen probe for BLOODPRG 0x00A7E6.
 * This models the callers as fixed-size record assignments. The Watcom-only
 * pragma states the observed helper ABI; it does not supply instruction code.
 * This is not recovered game source.
 */
typedef unsigned int u16;

typedef struct word_block {
    u16 words[4];
} word_block;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

void NEAR mem_copy_words_probe(word_block *dst, const word_block *src);

#if defined(__WATCOMC__)
#pragma aux mem_copy_words_probe parm [di] [si] modify exact [es di si]
#endif

void NEAR mem_copy_words_probe(word_block *dst, const word_block *src)
{
    *dst = *src;
}
