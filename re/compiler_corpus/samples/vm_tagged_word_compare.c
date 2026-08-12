/*
 * Codegen probe for BLOODPRG 0x0064E5.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile i16 compare_word;
u16 NEAR vm_branch_probe(void);

#if defined(__WATCOMC__)
#pragma aux vm_branch_probe value [si] modify exact [ax si]
#pragma aux vm_tagged_word_compare_probe parm [si] value [si] modify exact [ax dx si]
#endif

const u16 NEAR *NEAR vm_tagged_word_compare_probe(
        const u16 NEAR *script_words)
{
    u8 tag;
    i16 value;

    tag = (u8)*script_words++;
    value = (i16)*script_words++;

    if (tag == 0xf1u) {
        if (value > compare_word) {
            return script_words;
        }
    } else if (tag == 0xf2u) {
        if (value < compare_word) {
            return script_words;
        }
    } else if (value == compare_word) {
        return script_words;
    }

    return (const u16 NEAR *)vm_branch_probe();
}
