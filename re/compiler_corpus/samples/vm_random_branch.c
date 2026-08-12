/*
 * Codegen probe for BLOODPRG 0x006588.
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

u16 FAR prng_next(u16 modulus);
u16 NEAR vm_branch_probe(void);

#if defined(__WATCOMC__)
#pragma aux prng_next parm [ax] value [ax] modify exact [ax]
#pragma aux vm_branch_probe value [si] modify exact [ax si]
#pragma aux vm_random_branch_probe parm [si] value [si] modify exact [ax si]
#endif

const u16 NEAR *NEAR vm_random_branch_probe(
        const u16 NEAR *script_words)
{
    u16 modulus;

    modulus = *script_words++;
    if (prng_next(modulus) != 0) {
        return (const u16 NEAR *)vm_branch_probe();
    }

    return script_words;
}
